# AI-assisted (OpenBot agent)
"""Avaliação retrospectiva ponta a ponta (protocolo v1.1) sobre o snapshot real.

Uso (no contêiner):  python scripts/run_evaluation.py [--snapshot DIR] [--allow-config-change]

Grava reports/<run_id>/ com run_result.json, eval_report.json, eval_report.md, run_manifest.json e
SHA256SUMS, e acrescenta uma linha em reports/eval_runs.md (TODAS as execuções, inclusive falhas).
A primeira execução congela reports/FROZEN_CONFIG.json; as seguintes recusam config diferente.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import platform
import subprocess
import sys
import time
import traceback
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

from fieldshift import catalog as cat  # noqa: E402
from fieldshift import data, evaluation, indicators, optimization  # noqa: E402

REPORTS = ROOT / "reports"
CONFIG = ROOT / "config"


def sha256_file(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def git_state() -> dict[str, str | bool | None]:
    """Commit lido de .git (o contêiner não tem o binário git); ``dirty`` só se o git existir."""
    head = ROOT / ".git" / "HEAD"
    commit = "indisponivel"
    try:
        ref = head.read_text().strip()
        if ref.startswith("ref: "):
            rp = ROOT / ".git" / ref[5:]
            if rp.exists():
                commit = rp.read_text().strip()
            else:
                packed = (ROOT / ".git" / "packed-refs").read_text().splitlines()
                commit = next(ln.split()[0] for ln in packed if ln.endswith(ref[5:]))
        else:
            commit = ref
    except Exception:  # noqa: BLE001
        pass
    try:
        st = subprocess.run(
            ["git", "status", "--porcelain"], cwd=ROOT, capture_output=True, text=True, check=True
        ).stdout
        dirty: bool | None = bool(st.strip())
    except Exception:  # noqa: BLE001
        dirty = None  # desconhecido no contêiner
    return {"commit": commit, "dirty": dirty}


def now() -> str:
    return dt.datetime.now(dt.UTC).strftime("%Y-%m-%dT%H:%M:%SZ")


def frozen_config(rr: optimization.RunResult, proto: evaluation.EvalProtocol) -> dict:
    return {
        "protocol_version": proto.version,
        "protocol_file_sha256": proto.sha256,
        "protocol_config_hash": proto.config_hash(),
        "optimizer_config_hash": rr.config_hash,
        "snapshot_id": rr.snapshot_id,
        "soil": rr.soil,
        "weights": rr.weights,
        "weight_grid": rr.weight_grid,
        "p0_spec": (rr.policies.get("P0") or {}).get("spec"),
        "seed": proto.seed,
        "epsilon": proto.epsilon,
    }


def append_log(line: str) -> None:
    log = REPORTS / "eval_runs.md"
    if not log.exists():
        log.write_text(
            "# Registro de execuções da avaliação (protocolo v1.1)\n\n"
            "Todas as execuções que tocam o teste, inclusive falhas e reexecuções.\n\n"
            "| run_id | início (UTC) | estado | desfecho | eval result_hash | run_result_hash "
            "| config congelada | commit (dirty) |\n|---|---|---|---|---|---|---|---|\n",
            encoding="utf-8",
        )
    with log.open("a", encoding="utf-8") as f:
        f.write(line + "\n")


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--snapshot", type=Path, default=None)
    ap.add_argument("--allow-config-change", action="store_true")
    ap.add_argument("--n-boot", type=int, default=None, help="só para testes; padrão = protocolo")
    args = ap.parse_args()

    REPORTS.mkdir(exist_ok=True)
    started = now()
    t0 = time.perf_counter()
    git = git_state()
    run_id = dt.datetime.now(dt.UTC).strftime("%Y%m%dT%H%M%SZ")
    out = REPORTS / run_id
    try:
        proto = evaluation.EvalProtocol()
        if args.n_boot:
            proto = evaluation.EvalProtocol(n_boot=args.n_boot)
        proto_file = sha256_file(ROOT / "docs" / "EVALUATION_PROTOCOL.md")
        if proto_file != evaluation.PROTOCOL_SHA256:
            raise evaluation.EvaluationError(
                f"docs/EVALUATION_PROTOCOL.md mudou ({proto_file}); exige v1.2 registrada"
            )

        snap = args.snapshot or data.latest_snapshot(ROOT / "data" / "snapshots")
        manifest = data.verify_manifest(snap)
        climate = data.load_climate(snap)

        catalog = cat.load_catalog(CONFIG)
        soil = cat.load_soil(CONFIG)
        params = indicators.load_indicator_params(CONFIG, catalog)

        t_opt = time.perf_counter()
        rr = optimization.run(climate, catalog, soil, params, snapshot_id=manifest["snapshot_id"])
        t_opt = time.perf_counter() - t_opt

        fc = frozen_config(rr, proto)
        fc_hash = evaluation.sha256_json(fc)
        frozen_path = REPORTS / "FROZEN_CONFIG.json"
        if not frozen_path.exists():
            frozen_path.write_text(
                json.dumps(
                    {"frozen_utc": started, "config": fc, "sha256": fc_hash},
                    indent=2,
                    ensure_ascii=False,
                )
                + "\n",
                encoding="utf-8",
            )
        frozen = json.loads(frozen_path.read_text(encoding="utf-8"))
        if frozen["sha256"] != fc_hash and not args.allow_config_change:
            raise evaluation.EvaluationError(
                f"configuração difere da congelada ({frozen['sha256'][:12]} != {fc_hash[:12]})"
            )

        # O avaliador enumera X por conta própria e confere contra o otimizador.
        plans, _rej = cat.enumerate_plans(catalog, soil, 0.20)
        t_ev = time.perf_counter()
        if rr.status == "INVIAVEL":
            plans = []
        report = (
            evaluation.evaluate(
                rr,
                climate,
                proto,
                plans=plans,
                loss_model=evaluation.IndicatorLossModel(params),
                check_plan=lambda p: cat.verify_plan(catalog, soil, p),
                optimizer_params_hash=(rr.climatology or {}).get("precip_sha256"),
            )
            if rr.status == "OK"
            else evaluation.evaluate_plans(
                plans=plans,
                policy_plans={},
                climate=climate,
                loss_model=evaluation.IndicatorLossModel(params),
                protocol=proto,
            )
        )
        t_ev = time.perf_counter() - t_ev
        if rr.status == "OK":
            x_hash = hashlib.sha256(json.dumps([list(p.id) for p in plans]).encode()).hexdigest()
            report.audit["x_hash_match"] = x_hash == rr.x_hash
            report.audit["sbar_hash_match"] = evaluation.sbar_sha256_like_optimizer(
                climate, proto
            ) == rr.climatology.get("sbar_sha256")
            report.audit["optimizer_status"] = rr.status

        out.mkdir(parents=True)
        files = {
            "run_result.json": rr.to_json(),
            "eval_report.json": json.dumps(
                {**report.to_dict(), "result_hash": report.result_hash()},
                indent=2,
                sort_keys=True,
                ensure_ascii=False,
            ),
            "eval_report.md": report.to_markdown(),
        }
        for name, text in files.items():
            (out / name).write_text(text + ("" if text.endswith("\n") else "\n"), encoding="utf-8")
        run_manifest = {
            "run_id": run_id,
            "started_utc": started,
            "finished_utc": now(),
            "seconds": {
                "optimization": round(t_opt, 2),
                "evaluation": round(t_ev, 2),
                "total": round(time.perf_counter() - t0, 2),
            },
            "outcome": report.outcome,
            "eval_result_hash": report.result_hash(),
            "run_result_hash": rr.result_hash(),
            "frozen_config_sha256": fc_hash,
            "frozen_config_matches": frozen["sha256"] == fc_hash,
            "snapshot_id": manifest["snapshot_id"],
            "snapshot_manifest_sha256": sha256_file(Path(snap) / "manifest.json"),
            "protocol_file_sha256": proto_file,
            "git": git,
            "uv_lock_sha256": sha256_file(ROOT / "uv.lock")
            if (ROOT / "uv.lock").exists()
            else None,
            "versions": {
                "python": platform.python_version(),
                "numpy": np.__version__,
                "pandas": pd.__version__,
            },
            "seed": proto.seed,
            "n_boot": proto.n_boot,
        }
        (out / "run_manifest.json").write_text(
            json.dumps(run_manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
        sums = "".join(
            f"{sha256_file(out / n)}  {n}\n"
            for n in sorted(
                ["run_result.json", "eval_report.json", "eval_report.md", "run_manifest.json"]
            )
        )
        (out / "SHA256SUMS").write_text(sums, encoding="utf-8")
        append_log(
            f"| {run_id} | {started} | ok | {report.outcome} | `{report.result_hash()}` "
            f"| `{rr.result_hash()}` | `{fc_hash[:12]}` "
            f"({'igual' if frozen['sha256'] == fc_hash else 'DIFERENTE'}) "
            f"| `{str(git['commit'])[:7]}` ({git['dirty']}) |"
        )
        print(report.to_markdown().splitlines()[0])
        print(json.dumps(run_manifest, indent=2, ensure_ascii=False))
        return 0
    except Exception as exc:  # noqa: BLE001
        append_log(
            f"| {run_id} | {started} | FALHA | — | — | — | — "
            f"| `{str(git['commit'])[:7]}` ({git['dirty']}) |"
        )
        traceback.print_exc()
        print(f"FALHA: {exc}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
