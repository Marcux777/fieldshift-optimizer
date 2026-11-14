# AI-assisted (OpenBot agent)
"""E0: gera as fixtures de referência (oráculo) da migração Rust a partir do Python do baseline.

SÓ PARA TESTE. Roda no contêiner ``fieldshift-oracle`` (sem rede), sobre o código Python desta
worktree, que deve ser idêntico ao commit 16cc391. As fixtures são oráculos dos testes
diferenciais em Rust; o produto Rust NUNCA as lê para produzir respostas.

Uso:  docker compose -f compose.oracle.yaml exec -T oracle python tools/reference/dump_reference.py
Saída: fixtures/reference/*.json e fixtures/reference/MANIFEST.json (SHA-256 de cada arquivo).
Floats são gravados com ``repr`` do Python (ida e volta exata via json).
"""

from __future__ import annotations

import hashlib
import json
import math
import platform
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "src"))

import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

from fieldshift import catalog as cat  # noqa: E402
from fieldshift import data, evaluation, indicators, optimization  # noqa: E402

BASELINE = "16cc391318b57fbf44a31c75c994fbb0683b3115"
OUT = ROOT / "fixtures" / "reference"
CONFIG = ROOT / "config"
GOLDEN_RUN = ROOT / "reports" / "20261114T184118Z"
PY_SOURCES = [
    "src/fieldshift/catalog.py",
    "src/fieldshift/data.py",
    "src/fieldshift/evaluation.py",
    "src/fieldshift/indicators.py",
    "src/fieldshift/optimization.py",
]


def plain(x):
    """Converte para tipos JSON nativos sem perder bits (floats ficam float64)."""
    if isinstance(x, dict):
        return {str(k): plain(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [plain(v) for v in x]
    if isinstance(x, np.ndarray):
        return plain(x.tolist())
    if isinstance(x, (np.floating,)):
        return float(x)
    if isinstance(x, (np.integer,)):
        return int(x)
    if isinstance(x, (np.bool_,)):
        return bool(x)
    if isinstance(x, float) and not math.isfinite(x):
        raise ValueError("valor não finito na fixture")
    return x


def write(name: str, obj) -> None:
    text = json.dumps(plain(obj), sort_keys=True, ensure_ascii=False, allow_nan=False)
    (OUT / name).write_text(text + "\n", encoding="utf-8")


def sha256_file(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def occupation(o) -> dict:
    return {
        "window": o.window,
        "crop": o.crop,
        "sow_t": o.sow_t,
        "emerge_t": o.emerge_t,
        "harvest_t": o.harvest_t,
        "dec": o.dec,
    }


def dump_catalogs(catalog) -> dict:
    out = {}
    soils = [(ad, "argilosa") for ad in ("AD1", "AD2", "AD3", "AD4", "AD5", "AD6")]
    soils += [("AD4", "arenosa"), ("AD4", "media")]
    for ad, tx in soils:
        soil = cat.load_soil(CONFIG, ad, tx)
        plans, rej = cat.enumerate_plans(catalog, soil, 0.20)
        p0 = cat.default_p0_spec(catalog)
        p0_plan, p0_reasons = cat.resolve_spec(catalog, soil, p0, plans)
        verify = {str(i): cat.verify_plan(catalog, soil, p) for i, p in enumerate(plans)}
        name = f"catalog_{ad}_{tx}.json"
        write(
            name,
            {
                "soil": soil.describe(),
                "n_plans": len(plans),
                "plans": [
                    {
                        "id": list(p.id),
                        "signature": list(p.signature),
                        "occupations": [occupation(o) for o in p.occupations],
                        "fallow_windows": list(p.fallow_windows()),
                    }
                    for p in plans
                ],
                "rejections": [r.to_dict() for r in rej],
                "rejections_summary": cat.summarize_rejections(rej),
                "fallow_summary": cat.fallow_summary(plans),
                "diagnosis_if_empty": cat.diagnose_infeasible(rej) if not plans else None,
                "p0_spec": p0.to_dict(),
                "p0_plan_id": list(p0_plan.id) if p0_plan is not None else None,
                "p0_reasons": p0_reasons,
                "verify_plan_violations_nonempty": [k for k, v in verify.items() if v],
            },
        )
        out[f"{ad}_{tx}"] = len(plans)
    return out


def main() -> int:
    OUT.mkdir(parents=True, exist_ok=True)
    head_ok = {}
    for src in PY_SOURCES:
        try:
            blob = subprocess.run(
                ["git", "show", f"{BASELINE}:{src}"], cwd=ROOT, capture_output=True, check=True
            ).stdout
            head_ok[src] = hashlib.sha256(blob).hexdigest() == sha256_file(ROOT / src)
        except Exception:  # noqa: BLE001  (o contêiner não tem git)
            head_ok[src] = None
    src_sha = {s: sha256_file(ROOT / s) for s in PY_SOURCES}

    snap = data.latest_snapshot(ROOT / "data" / "snapshots")
    manifest = data.verify_manifest(snap)
    climate = data.load_climate(snap)
    catalog = cat.load_catalog(CONFIG)
    soil = cat.load_soil(CONFIG)
    params = indicators.load_indicator_params(CONFIG, catalog)

    counts = dump_catalogs(catalog)

    # --- otimização (RunResult real, recalculado aqui) --------------------------------------
    rr = optimization.run(climate, catalog, soil, params, snapshot_id=manifest["snapshot_id"])
    rr_hash = rr.result_hash()
    plans, _ = cat.enumerate_plans(catalog, soil, 0.20)
    ids = [p.id for p in plans]

    cset = optimization.build_scenarios(climate, "construction")
    tset = optimization.build_scenarios(climate, "test")
    ref, sbar = optimization.construction_climatology(cset)
    prm = params.with_reference(ref)
    cpairs = [s.pair for s in cset.valid]
    tpairs = [s.pair for s in tset.valid]
    comp_c = optimization.components_tensor(plans, cpairs, prm)
    comp_t = optimization.components_tensor(plans, tpairs, prm)
    comp_b = optimization.components_tensor(plans, [sbar], prm)[:, 0, :]

    # medianas de e3 usadas (memo da climatologia), na ordem das chaves
    e3_medians = {f"{lo}:{hi}": v for (lo, hi), v in sorted(ref._memo.items())}

    write(
        "scenarios.json",
        {
            "construction": [
                {"y": s.y, "label": s.label, "valid": s.valid, "reason": s.reason}
                for s in cset.scenarios
            ],
            "test": [
                {"y": s.y, "label": s.label, "valid": s.valid, "reason": s.reason}
                for s in tset.scenarios
            ],
            "climatology_years": list(ref.labels),
            "climatology_precip_sha256": ref.sha256(),
            "sbar": {"P": sbar.P, "TX": sbar.TX, "TN": sbar.TN},
            "sbar_sha256": rr.climatology["sbar_sha256"],
            "e3_median_window_sums": e3_medians,
            "construction_pairs": [
                {"label": s.label, "P": s.pair.P, "TX": s.pair.TX, "TN": s.pair.TN}
                for s in cset.valid[:2]
            ],
        },
    )
    write(
        "components_default.json",
        {
            "soil": soil.describe(),
            "plan_ids": [list(i) for i in ids],
            "construction_labels": [s.label for s in cset.valid],
            "test_labels": [s.label for s in tset.valid],
            "construction": comp_c,
            "test": comp_t,
            "sbar": comp_b,
        },
    )

    # seleções nos 15 pesos, com as chaves numéricas (antes e depois de q)
    sel_rows = []
    for w in optimization.WEIGHT_GRID:
        L = optimization.loss_matrix(comp_c, w)
        Lb = optimization.loss_matrix(comp_b, w)
        R = optimization.regret_matrix(L)
        row = {"weights": list(w)}
        for name, s in (
            ("P1", optimization.select_conv(L, Lb, ids)),
            ("P1b", optimization.select_exp(L, ids)),
            ("P2", optimization.select_mmr(L, ids)),
        ):
            row[name] = {
                "index": s.index,
                "plan_id": list(ids[s.index]),
                "tied_after_level": list(s.tied_after_level),
                "id_tiebreak": s.id_tiebreak,
            }
        row["keys"] = {
            "max_R": R.max(axis=1),
            "mean_R_fsum": optimization.mean_over_scenarios(R),
            "mean_L_fsum": optimization.mean_over_scenarios(L),
            "L_sbar": Lb,
        }
        sel_rows.append(row)
    write("selections_15w.json", {"plan_ids": [list(i) for i in ids], "rows": sel_rows})

    F = optimization.objective_vectors(comp_c)
    front, equivalents = optimization.pareto_front(F, ids)
    write(
        "pareto.json",
        {
            "F": F,
            "front_indices": front,
            "front_ids": [list(ids[i]) for i in front],
            "equivalents": {str(k): v for k, v in equivalents.items()},
            "n_front_vectors": len(front),
            "n_front_plans": len(front) + sum(len(v) for v in equivalents.values()),
            "alternatives": rr.alternatives,
        },
    )

    # --- avaliação (intermediários) ----------------------------------------------------------
    proto = evaluation.EvalProtocol()
    split = evaluation.split_scenarios(climate, proto)
    model = evaluation.IndicatorLossModel(params)
    eparams = model.fit(split.construction)
    s_bar = evaluation.mean_climate(split.construction_years)
    p25 = evaluation.dry_threshold(split.construction_years, proto)
    G_c = evaluation.component_matrix(plans, split.construction, model, eparams)
    G_t = evaluation.component_matrix(plans, split.test, model, eparams)
    w = proto.default_weights
    L_t = evaluation.losses(G_t, w)
    R_t = evaluation.regret(L_t)
    dry = [
        list(evaluation.scenario_dry_season_totals(sc, proto)) for sc in split.test
    ]
    nb = math.ceil(len(split.test) / proto.block_size)
    rng = np.random.default_rng(proto.seed)
    boot_idx = rng.integers(0, nb, size=(proto.n_boot, nb))

    report = evaluation.evaluate(
        rr,
        climate,
        proto,
        plans=plans,
        loss_model=evaluation.IndicatorLossModel(params),
        check_plan=lambda p: cat.verify_plan(catalog, soil, p),
        optimizer_params_hash=rr.climatology["precip_sha256"],
    )
    report.audit["x_hash_match"] = (
        hashlib.sha256(json.dumps([list(p.id) for p in plans]).encode()).hexdigest() == rr.x_hash
    )
    report.audit["sbar_hash_match"] = (
        evaluation.sbar_sha256_like_optimizer(climate, proto) == rr.climatology["sbar_sha256"]
    )
    report.audit["optimizer_status"] = rr.status
    ev_hash = report.result_hash()

    write(
        "eval_intermediates.json",
        {
            "construction_labels": [s.label for s in split.construction],
            "test_labels": [s.label for s in split.test],
            "construction_years": [y.start for y in split.construction_years],
            "dry_threshold_mm": p25,
            "dry_season_totals_test": dry,
            "construction_dry_season_totals": [
                evaluation._dry_season_total(y, proto) for y in split.construction_years
            ],
            "mean_climate_hash": evaluation._array_hash(s_bar.precip, s_bar.tmax, s_bar.tmin),
            "params_hash": model.fingerprint(eparams),
            "L_test_default": L_t,
            "R_test_default": R_t,
            "components_equal_optimizer": bool(
                np.array_equal(G_c, comp_c) and np.array_equal(G_t, comp_t)
            ),
            "bootstrap": {
                "seed": proto.seed,
                "n_boot": proto.n_boot,
                "n_blocks": nb,
                "indices": boot_idx,
            },
            "report_dict": report.to_dict(),
        },
    )

    # --- clima diário e componentes por CONTEÚDO de cenário (inclui s̄ do leave-one-out) ---------
    write(
        "climate_daily.json",
        {
            "dates": [d.strftime("%Y-%m-%d") for d in climate.index],
            "PRECTOTCORR": climate["PRECTOTCORR"].to_numpy(),
            "T2M": climate["T2M"].to_numpy(),
            "T2M_MAX": climate["T2M_MAX"].to_numpy(),
            "T2M_MIN": climate["T2M_MIN"].to_numpy(),
        },
    )

    def content_key(sc) -> str:
        h = hashlib.sha256()
        for a in (sc.precip, sc.tmax, sc.tmin):
            h.update(np.ascontiguousarray(a, dtype=np.float64).tobytes())
        return h.hexdigest()

    by_content = {}
    for sc, j in zip(split.construction, range(len(split.construction)), strict=True):
        by_content[content_key(sc)] = {"label": sc.label, "g": G_c[:, j, :]}
    for sc, j in zip(split.test, range(len(split.test)), strict=True):
        by_content[content_key(sc)] = {"label": sc.label, "g": G_t[:, j, :]}
    years_by_start = {y.start: y for y in split.construction_years}
    n = len(split.construction)
    loo_sets = {}
    for k in [None, *range(n)]:
        keep = [j for j in range(n) if j != k]
        yrs = tuple(sorted({y for j in keep for y in split.construction[j].years}))
        if yrs in loo_sets:
            continue
        sb = evaluation.mean_climate([years_by_start[y] for y in yrs])
        g = evaluation.component_matrix(plans, [sb], model, eparams)[:, 0, :]
        loo_sets[yrs] = content_key(sb)
        by_content[content_key(sb)] = {"label": f"sbar:{yrs[0]}-{yrs[-1]}:{len(yrs)}", "g": g}
    write(
        "components_by_content.json",
        {
            "plan_ids": [list(i) for i in ids],
            "key": "sha256(float64 bytes P || TX || TN), 730 dias",
            "loo_year_sets": {",".join(map(str, k)): v for k, v in loo_sets.items()},
            "scenarios": by_content,
        },
    )

    # --- goldens congelados (cópias exatas) ----------------------------------------------
    for name in ("run_result.json", "eval_report.json"):
        shutil.copyfile(GOLDEN_RUN / name, OUT / f"golden_{name}")
    golden_rr = json.loads((GOLDEN_RUN / "run_result.json").read_text())["result_hash"]
    golden_ev = json.loads((GOLDEN_RUN / "eval_report.json").read_text())["result_hash"]

    files = sorted(p for p in OUT.glob("*.json") if p.name != "MANIFEST.json")
    manifest_out = {
        "purpose": "Oráculo de TESTE da migração Rust. Proibido usar como resposta do produto.",
        "baseline_commit": BASELINE,
        "python_sources_sha256": src_sha,
        "python_sources_equal_baseline": head_ok,
        "command": "docker compose -f compose.oracle.yaml exec -T oracle "
        "python tools/reference/dump_reference.py",
        "versions": {
            "python": platform.python_version(),
            "numpy": np.__version__,
            "pandas": pd.__version__,
        },
        "snapshot_id": manifest["snapshot_id"],
        "protocol_sha256": evaluation.PROTOCOL_SHA256,
        "plan_counts": counts,
        "recomputed": {"run_result_hash": rr_hash, "eval_report_hash": ev_hash},
        "golden": {"run_result_hash": golden_rr, "eval_report_hash": golden_ev},
        "recomputed_equals_golden": rr_hash == golden_rr and ev_hash == golden_ev,
        "files": {p.name: sha256_file(p) for p in files},
    }
    (OUT / "MANIFEST.json").write_text(
        json.dumps(manifest_out, indent=2, sort_keys=True, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    print(json.dumps({k: manifest_out[k] for k in ("plan_counts", "recomputed",
                                                    "recomputed_equals_golden")}, indent=2))
    return 0 if manifest_out["recomputed_equals_golden"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
