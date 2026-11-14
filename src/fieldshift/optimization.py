# AI-assisted (OpenBot agent)
"""Cenários, políticas (P0, P1, P1b, P2), minimax regret, Pareto e ``RunResult``.

Enumeração exaustiva (|X| ~ 10^2–10^3): ótimo e fronteira exatos, sem solver. Núcleo sem I/O: recebe
o DataFrame de ``data.load_climate`` (índice ``date``; PRECTOTCORR, T2M, T2M_MAX, T2M_MIN).

Protocolo v1.1 (``docs/EVALUATION_PROTOCOL.md``):

* cenário = par de anos agrícolas consecutivos (y, y+1), de 1/set a 31/ago, sem 29/fev (730 dias);
  construção: pares y = 1981..2008 (28); teste: y = 2010..2023 (14); o par 2009 não existe (cruzaria
  a fronteira). **O teste nunca entra na escolha**: a construção é fatiada do DataFrame antes de
  qualquer cálculo e as escolhas só recebem tensores da construção (teste T2).
* cenário inválido (NaN em PRECTOTCORR, T2M, T2M_MAX ou T2M_MIN, ou dia ausente) sai para TODAS
  as políticas; menos de 20 pares válidos na construção ou 10 no teste => ``DADOS_INSUFICIENTES``.
* L_w(x, s) = w1 g1 + w2 g2 + w3 g3;  R(x, s) = L(x, s) - min_x' L(x', s);  q(v) = round(v, 9).
* Desempate lexicográfico, depois menor ID do plano: P2 (minimax regret): max R, média R;
  P1 (clima médio): L(x, s_barra), média L;  P1b (esperança): média L, max R.
* P0 fora de X (viola ZARC <= 20% ou regra de calendário) => "inviável sob ZARC", sai do regret.
* X vazio => ``INVIAVEL`` com diagnóstico; nada é relaxado.
"""

from __future__ import annotations

import hashlib
import json
import math
from collections.abc import Sequence
from dataclasses import dataclass, field
from typing import Any

import numpy as np
import pandas as pd

from .catalog import (
    DAYS_PER_YEAR,
    HORIZON_DAYS,
    ZARC_RISK_LIMIT_PCT,
    Catalog,
    Plan,
    PlanSpec,
    Soil,
    default_p0_spec,
    describe_plan,
    diagnose_infeasible,
    enumerate_plans,
    fallow_summary,
    resolve_spec,
    summarize_rejections,
)
from .indicators import ClimatePair, ConstructionClimatology, IndicatorParams, loss_components

# --------------------------------------------------------------------------- protocolo v1.1
PROTOCOL_VERSION = "1.1"
CONSTRUCTION_FIRST_Y = 1981
CONSTRUCTION_LAST_PAIR_Y = 2008  # par (2008/09, 2009/10)
CONSTRUCTION_LAST_YEAR = 2009  # último ano agrícola de construção: 2009/10
TEST_FIRST_Y = 2010
TEST_LAST_PAIR_Y = 2023  # par (2023/24, 2024/25)
CONSTRUCTION_END = pd.Timestamp("2010-08-31")
TEST_START = pd.Timestamp("2010-09-01")
MIN_VALID_CONSTRUCTION = 20
MIN_VALID_TEST = 10
ROUND = 9
K_ALTERNATIVES = 6
MIN_HAMMING = 2
VARIABLES = ("PRECTOTCORR", "T2M", "T2M_MAX", "T2M_MIN")
POLICIES = ("P0", "P1", "P1b", "P2")


def _grid() -> tuple[tuple[float, float, float], ...]:
    out = []
    for a in range(4, -1, -1):
        for b in range(4 - a, -1, -1):
            out.append((a / 4, b / 4, (4 - a - b) / 4))
    return tuple(out)


WEIGHT_GRID: tuple[tuple[float, float, float], ...] = _grid()  # (exposição, cobertura, diversidade)
DEFAULT_WEIGHTS = (0.50, 0.25, 0.25)
assert len(WEIGHT_GRID) == 15 and DEFAULT_WEIGHTS in WEIGHT_GRID


class OptimizationError(ValueError):
    """Entrada inválida ou estado impossível (falha alta; sem fallback silencioso)."""


class ClimatologyError(OptimizationError):
    """Algum dia de s_barra ficou sem valor: a execução aborta (sem imputação)."""


# --------------------------------------------------------------------------- cenários
@dataclass(frozen=True)
class AgYear:
    y: int
    P: np.ndarray
    TX: np.ndarray
    TN: np.ndarray
    missing: dict[str, int]

    @property
    def complete(self) -> bool:
        return not any(self.missing.values())


@dataclass(frozen=True)
class Scenario:
    y: int
    label: str
    split: str
    valid: bool
    reason: str
    pair: ClimatePair | None


@dataclass(frozen=True)
class ScenarioSet:
    split: str
    scenarios: tuple[Scenario, ...]
    ag_years: dict[int, AgYear]
    n_expected: int

    @property
    def valid(self) -> list[Scenario]:
        return [s for s in self.scenarios if s.valid]

    def missing_log(self) -> list[dict[str, Any]]:
        return [
            {"ag_year": y, **{k: v for k, v in a.missing.items() if v}}
            for y, a in sorted(self.ag_years.items())
            if not a.complete
        ]

    def excluded(self) -> list[dict[str, Any]]:
        return [
            {"label": s.label, "y": s.y, "reason": s.reason} for s in self.scenarios if not s.valid
        ]

    def summary(self) -> dict[str, Any]:
        return {
            "n_expected": self.n_expected,
            "n_valid": len(self.valid),
            "excluded": self.excluded(),
            "missing_log": self.missing_log(),
        }


def _agri_year_frame(climate: pd.DataFrame, y: int) -> AgYear:
    idx = pd.date_range(f"{y}-09-01", f"{y + 1}-08-31", freq="D")
    idx = idx[~((idx.month == 2) & (idx.day == 29))]  # 29/fev descartado: 365 dias
    if len(idx) != DAYS_PER_YEAR:
        raise OptimizationError("ano agrícola deveria ter 365 dias")
    sub = climate.reindex(idx)  # dia ausente vira NaN
    missing = {v: int(sub[v].isna().sum()) for v in VARIABLES}
    return AgYear(
        y,
        sub["PRECTOTCORR"].to_numpy(dtype=np.float64),
        sub["T2M_MAX"].to_numpy(dtype=np.float64),
        sub["T2M_MIN"].to_numpy(dtype=np.float64),
        missing,
    )


def _check_columns(climate: pd.DataFrame) -> None:
    miss = [c for c in VARIABLES if c not in climate.columns]
    if miss:
        raise OptimizationError(f"clima sem colunas: {miss}")
    if not isinstance(climate.index, pd.DatetimeIndex) or not climate.index.is_unique:
        raise OptimizationError("índice do clima deve ser DatetimeIndex único")


def build_scenarios(climate: pd.DataFrame, split: str) -> ScenarioSet:
    """Cenários do split ('construction' | 'test'). A fatia é feita ANTES de qualquer cálculo."""
    _check_columns(climate)
    if split == "construction":
        frame = climate.loc[:CONSTRUCTION_END]
        years = range(CONSTRUCTION_FIRST_Y, CONSTRUCTION_LAST_YEAR + 1)
        pair_ys = range(CONSTRUCTION_FIRST_Y, CONSTRUCTION_LAST_PAIR_Y + 1)
        if len(frame) and frame.index.max() >= TEST_START:
            raise OptimizationError("dado de teste na construção")
    elif split == "test":
        frame = climate.loc[TEST_START:]
        years = range(TEST_FIRST_Y, TEST_LAST_PAIR_Y + 2)
        pair_ys = range(TEST_FIRST_Y, TEST_LAST_PAIR_Y + 1)
    else:
        raise OptimizationError(f"split desconhecido: {split!r}")
    ag = {y: _agri_year_frame(frame, y) for y in years}
    scen: list[Scenario] = []
    for y in pair_ys:
        a, b = ag[y], ag[y + 1]
        label = f"{y}/{(y + 1) % 100:02d}+{y + 1}/{(y + 2) % 100:02d}"
        if a.complete and b.complete:
            pair = ClimatePair(
                np.concatenate([a.P, b.P]),
                np.concatenate([a.TX, b.TX]),
                np.concatenate([a.TN, b.TN]),
                label,
            )
            scen.append(Scenario(y, label, split, True, "", pair))
        else:
            bad = {
                k: v
                for k, v in {
                    **{f"{y}:{k}": n for k, n in a.missing.items()},
                    **{f"{y + 1}:{k}": n for k, n in b.missing.items()},
                }.items()
                if v
            }
            scen.append(Scenario(y, label, split, False, f"dados ausentes {bad}", None))
    return ScenarioSet(split, tuple(scen), ag, len(pair_ys))


def construction_climatology(sset: ScenarioSet) -> tuple[ConstructionClimatology, ClimatePair]:
    """(clima de referência de e3, s_barra) só com anos dos pares válidos da construção."""
    if sset.split != "construction":
        raise OptimizationError("a climatologia só pode vir da construção")
    years = sorted({s.y for s in sset.valid} | {s.y + 1 for s in sset.valid})
    if not years:
        raise OptimizationError("sem anos válidos na construção")
    P = np.stack([sset.ag_years[y].P for y in years])
    TX = np.stack([sset.ag_years[y].TX for y in years])
    TN = np.stack([sset.ag_years[y].TN for y in years])
    sbar = [np.concatenate([m.mean(axis=0)] * 2) for m in (P, TX, TN)]
    if not all(np.isfinite(a).all() for a in sbar):
        raise ClimatologyError(
            "algum dia de s_barra ficou sem valor; execução abortada (sem imputação)"
        )
    return ConstructionClimatology(P, tuple(years)), ClimatePair(*sbar, label="s_barra")


def components_tensor(
    plans: Sequence[Plan], pairs: Sequence[ClimatePair], params: IndicatorParams
) -> np.ndarray:
    """(n_planos, n_pares, 3) com g1, g2, g3."""
    out = np.empty((len(plans), len(pairs), 3), dtype=np.float64)
    for j, pair in enumerate(pairs):
        for i, plan in enumerate(plans):
            out[i, j, :] = loss_components(plan, pair, params)
    return out


# ------------------------------------------------------------------ perdas, regret, seleção
def q(a: Any) -> Any:
    """Comparação numérica única: arredondamento a 9 casas."""
    return np.round(a, ROUND)


def loss_matrix(comp: np.ndarray, w: Sequence[float]) -> np.ndarray:
    """L = w1 g1 + w2 g2 + w3 g3, sempre na mesma ordem de soma. comp[..., 3] -> L[...]."""
    return w[0] * comp[..., 0] + w[1] * comp[..., 1] + w[2] * comp[..., 2]


def regret_matrix(L: np.ndarray) -> np.ndarray:
    """R[i, s] = L[i, s] - min_i L[i, s]  (>= 0; zero no oráculo de cada cenário)."""
    return L - L.min(axis=0, keepdims=True)


def mean_over_scenarios(M: np.ndarray) -> np.ndarray:
    """Média por plano com fsum (independe da ordem dos cenários)."""
    n = M.shape[1]
    return np.array([math.fsum(row) / n for row in M])


@dataclass(frozen=True)
class Selection:
    index: int
    tied_after_level: tuple[int, ...]  # nº de planos ainda empatados após cada critério
    id_tiebreak: bool  # True se foi preciso o ID para desempatar


def lexicographic_min(keys: Sequence[np.ndarray], ids: Sequence[tuple[int, ...]]) -> Selection:
    cand = np.arange(len(ids))
    tied: list[int] = []
    for k in keys:
        kk = k[cand]
        cand = cand[kk == kk.min()]
        tied.append(int(len(cand)))
        if len(cand) == 1:
            break
    pick = min(cand.tolist(), key=lambda i: ids[i])
    return Selection(int(pick), tuple(tied), len(cand) > 1)


def select_mmr(L: np.ndarray, ids: Sequence[tuple[int, ...]]) -> Selection:
    R = regret_matrix(L)
    return lexicographic_min([q(R.max(axis=1)), q(mean_over_scenarios(R))], ids)


def select_conv(L: np.ndarray, Lbar: np.ndarray, ids: Sequence[tuple[int, ...]]) -> Selection:
    return lexicographic_min([q(Lbar), q(mean_over_scenarios(L))], ids)


def select_exp(L: np.ndarray, ids: Sequence[tuple[int, ...]]) -> Selection:
    R = regret_matrix(L)
    return lexicographic_min([q(mean_over_scenarios(L)), q(R.max(axis=1))], ids)


# --------------------------------------------------------------------------- Pareto e alternativas
def objective_vectors(comp: np.ndarray) -> np.ndarray:
    """F(x) = (média g1, max g1, média g2, média g3) sobre S_constr, arredondado (q)."""
    g1, g2, g3 = comp[..., 0], comp[..., 1], comp[..., 2]
    F = np.column_stack(
        [mean_over_scenarios(g1), g1.max(axis=1), mean_over_scenarios(g2), mean_over_scenarios(g3)]
    )
    return q(F)


def pareto_front(
    F: np.ndarray, ids: Sequence[tuple[int, ...]]
) -> tuple[list[int], dict[int, list[int]]]:
    """Índices não dominados (dominância estrita, um representante por vetor igual: menor ID)
    e, para cada representante, os índices com vetor idêntico."""
    groups: dict[tuple[float, ...], list[int]] = {}
    for i in range(len(F)):
        groups.setdefault(tuple(F[i].tolist()), []).append(i)
    reps = [min(g, key=lambda i: ids[i]) for g in groups.values()]
    U = F[reps]
    front: list[int] = []
    for a in range(len(reps)):
        le = (U[a] >= U).all(axis=1)
        lt = (U[a] > U).any(axis=1)
        if not (le & lt).any():
            front.append(reps[a])
    front.sort(key=lambda i: ids[i])
    equivalents = {
        r: sorted((i for i in groups[tuple(F[r].tolist())] if i != r), key=lambda i: ids[i])
        for r in front
    }
    return front, equivalents


def hamming(a: Sequence[int], b: Sequence[int]) -> int:
    return sum(x != y for x, y in zip(a, b, strict=True))


def select_alternatives(
    plans: Sequence[Plan],
    F: np.ndarray,
    front: Sequence[int],
    policy_picks: Sequence[tuple[str, int]],
    w: Sequence[float],
    k: int = K_ALTERNATIVES,
    delta: int = MIN_HAMMING,
) -> list[dict[str, Any]]:
    """Alternativas distintas: assinatura (cultura de cada janela) sem repetição.

    1) planos das políticas (P0, P1, P1b, P2), um por assinatura (rótulos agrupados);
    2) extremos da fronteira (menor média de g1, g2, g3) com distância de Hamming >= delta;
    3) preenchimento por maior distância mínima (Hamming, depois euclidiana em F) com Hamming >=
       delta, relaxando para 1 (com aviso) se faltar candidato. Nunca completa com duplicatas.
    """
    ids = [p.id for p in plans]
    front_set = {tuple(F[i].tolist()) for i in front}
    alts: list[dict[str, Any]] = []
    sigs: list[tuple[int, int, int, int]] = []

    def add(i: int, labels: list[str], reason: str, relaxed: bool = False) -> None:
        sigs.append(plans[i].signature)
        alts.append(
            {
                "plan_index": i,
                "plan_id": list(plans[i].id),
                "labels": labels,
                "reason": reason,
                "on_front": tuple(F[i].tolist()) in front_set,
                "hamming_relaxed": relaxed,
            }
        )

    for label, i in policy_picks:
        sig = plans[i].signature
        if sig in sigs:
            alts[sigs.index(sig)]["labels"].append(label)
        else:
            add(i, [label], "policy")
        alts[sigs.index(sig)].setdefault("policy_plan_ids", {})[label] = list(plans[i].id)

    # candidatos: representantes da fronteira, um por assinatura (menor w*·F, depois ID)
    wv = np.array([w[0], w[0], w[1], w[2]])  # só ordena candidatos; não é objetivo novo
    best: dict[tuple[int, int, int, int], int] = {}
    for i in front:
        s = plans[i].signature
        if s in sigs:
            continue
        key = (round(float(F[i] @ wv), ROUND), ids[i])
        if s not in best or key < (round(float(F[best[s]] @ wv), ROUND), ids[best[s]]):
            best[s] = i
    cands = sorted(best.values(), key=lambda i: ids[i])

    def min_ham(i: int) -> int:
        return min(hamming(plans[i].signature, s) for s in sigs) if sigs else 99

    def pick_extremes(thr: int, relaxed: bool) -> None:
        for col, name in ((0, "extreme_g1"), (2, "extreme_g2"), (3, "extreme_g3")):
            if len(alts) >= k or not cands:
                return
            pool = [i for i in cands if min_ham(i) >= thr]
            if not pool:
                continue
            i = min(pool, key=lambda j: (float(F[j, col]), ids[j]))
            add(i, [], name, relaxed)
            cands.remove(i)

    pick_extremes(delta, False)
    for thr, relaxed in ((delta, False), (1, True)):
        while len(alts) < k:
            pool = [i for i in cands if min_ham(i) >= thr]
            if not pool:
                break

            def dist(i: int) -> float:
                return (
                    min(float(np.linalg.norm(F[i] - F[a["plan_index"]])) for a in alts)
                    if alts
                    else 0.0
                )

            i = max(
                pool, key=lambda j: (min_ham(j), round(dist(j), ROUND), tuple(-x for x in ids[j]))
            )
            add(i, [], "spread", relaxed)
            cands.remove(i)
    for a in alts:
        a["min_hamming_to_others"] = min(
            (
                hamming(plans[a["plan_index"]].signature, s)
                for s in sigs
                if s != plans[a["plan_index"]].signature
            ),
            default=0,
        )
    return alts


# --------------------------------------------------------------------------- resultado
def _r(x: Any) -> Any:
    if isinstance(x, (float, np.floating)):
        v = float(x)
        if not math.isfinite(v):
            raise OptimizationError("valor não finito no resultado")
        return round(v, 12)
    if isinstance(x, (np.integer,)):
        return int(x)
    if isinstance(x, np.ndarray):
        return [_r(v) for v in x.tolist()]
    if isinstance(x, dict):
        return {str(k): _r(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [_r(v) for v in x]
    return x


@dataclass
class RunResult:
    """Resultado serializável. ``meta`` (tempos, versões) fica fora do hash de resultado."""

    status: str
    message: str
    snapshot_id: str
    config_hash: str
    protocol_version: str
    soil: dict[str, str]
    risk_limit_pct: int
    weights: list[float]
    weight_grid: list[list[float]]
    n_plans: int
    x_hash: str
    fallow: dict[str, int]
    rejections_summary: dict[str, int]
    rejections: list[dict[str, Any]]
    diagnosis: dict[str, Any]
    scenarios: dict[str, Any]
    climatology: dict[str, Any]
    policies: dict[str, Any]
    objectives: list[list[Any]]
    pareto: dict[str, Any]
    alternatives: list[dict[str, Any]]
    sensitivity: list[dict[str, Any]]
    fallow_usage: dict[str, Any]
    meta: dict[str, Any] = field(default_factory=dict)

    def to_dict(self, include_meta: bool = True) -> dict[str, Any]:
        d = {k: _r(v) for k, v in self.__dict__.items() if k != "meta"}
        if include_meta:
            d["meta"] = _r(self.meta)
        return d

    def result_hash(self) -> str:
        """SHA-256 do JSON canônico sem ``meta`` (sem tempos): duas execuções => mesmo hash."""
        blob = json.dumps(
            self.to_dict(include_meta=False),
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
            ensure_ascii=True,
        )
        return hashlib.sha256(blob.encode("ascii")).hexdigest()

    def to_json(self, indent: int | None = 2) -> str:
        d = self.to_dict()
        d["result_hash"] = self.result_hash()
        return json.dumps(d, sort_keys=True, indent=indent, allow_nan=False, ensure_ascii=False)

    @classmethod
    def from_dict(cls, d: dict[str, Any]) -> RunResult:
        keep = {k: v for k, v in d.items() if k != "result_hash"}
        return cls(**keep)


def compute_config_hash(
    catalog: Catalog,
    soil: Soil,
    params: IndicatorParams,
    weights: Sequence[float],
    p0: PlanSpec,
    k: int = K_ALTERNATIVES,
    delta: int = MIN_HAMMING,
) -> str:
    cfg = {
        "files": dict(catalog.files_sha256),
        "indicators": params.sha256,
        "soil": soil.describe(),
        "risk_limit_pct": ZARC_RISK_LIMIT_PCT,
        "weights": list(weights),
        "grid": [list(w) for w in WEIGHT_GRID],
        "p0": p0.to_dict(),
        "protocol": {
            "version": PROTOCOL_VERSION,
            "construction": [CONSTRUCTION_FIRST_Y, CONSTRUCTION_LAST_PAIR_Y],
            "test": [TEST_FIRST_Y, TEST_LAST_PAIR_Y],
            "min_valid": [MIN_VALID_CONSTRUCTION, MIN_VALID_TEST],
            "round": ROUND,
            "alternatives": [k, delta],
        },
    }
    blob = json.dumps(cfg, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(blob.encode()).hexdigest()


def _x_hash(plans: Sequence[Plan]) -> str:
    return hashlib.sha256(json.dumps([list(p.id) for p in plans]).encode()).hexdigest()


def _empty_result(status: str, message: str, **kw: Any) -> RunResult:
    base: dict[str, Any] = {
        "status": status,
        "message": message,
        "snapshot_id": "",
        "config_hash": "",
        "protocol_version": PROTOCOL_VERSION,
        "soil": {},
        "risk_limit_pct": ZARC_RISK_LIMIT_PCT,
        "weights": list(DEFAULT_WEIGHTS),
        "weight_grid": [list(w) for w in WEIGHT_GRID],
        "n_plans": 0,
        "x_hash": "",
        "fallow": {},
        "rejections_summary": {},
        "rejections": [],
        "diagnosis": {},
        "scenarios": {},
        "climatology": {},
        "policies": {},
        "objectives": [],
        "pareto": {},
        "alternatives": [],
        "sensitivity": [],
        "fallow_usage": {},
    }
    base.update(kw)
    return RunResult(**base)


def _policy_entry(
    catalog: Catalog,
    plans: Sequence[Plan],
    i: int,
    L: np.ndarray,
    Lbar: np.ndarray,
    sel: Selection | None,
) -> dict[str, Any]:
    R = regret_matrix(L)
    entry: dict[str, Any] = {
        "status": "OK",
        "plan": describe_plan(catalog, plans[i]),
        "metrics_construction": {
            "L_sbar": float(Lbar[i]),
            "mean_L": math.fsum(L[i]) / L.shape[1],
            "max_regret": float(R[i].max()),
            "mean_regret": math.fsum(R[i]) / R.shape[1],
        },
        "fallow_windows": list(plans[i].fallow_windows()),
    }
    if sel is not None:
        entry["ties"] = {
            "tied_after_level": list(sel.tied_after_level),
            "id_tiebreak": sel.id_tiebreak,
        }
    return entry


def run(
    climate: pd.DataFrame,
    catalog: Catalog,
    soil: Soil,
    params: IndicatorParams,
    *,
    snapshot_id: str,
    p0: PlanSpec | None = None,
    weights: Sequence[float] | None = None,
    k_alternatives: int = K_ALTERNATIVES,
    min_hamming: int = MIN_HAMMING,
) -> RunResult:
    """Otimização: X -> cenários de construção -> P0/P1/P1b/P2 -> Pareto -> grade de pesos."""
    w = tuple(float(x) for x in (weights or params.default_weights))
    if len(w) != 3 or min(w) < 0 or abs(sum(w) - 1.0) > 1e-12:
        raise OptimizationError("weights deve ter 3 valores >= 0 somando 1")
    p0 = p0 or default_p0_spec(catalog)
    cfg_hash = compute_config_hash(catalog, soil, params, w, p0, k_alternatives, min_hamming)
    common = {
        "snapshot_id": snapshot_id,
        "config_hash": cfg_hash,
        "soil": soil.describe(),
        "weights": list(w),
    }

    plans, rejections = enumerate_plans(catalog, soil, ZARC_RISK_LIMIT_PCT / 100)
    rej_dicts = [r.to_dict() for r in rejections]
    if not plans:
        diag = diagnose_infeasible(rejections)
        return _empty_result(
            "INVIAVEL",
            diag["message"],
            diagnosis=diag,
            rejections=rej_dicts,
            rejections_summary=summarize_rejections(rejections),
            **common,
        )

    cset = build_scenarios(climate, "construction")
    tset = build_scenarios(
        climate, "test"
    )  # só contagem de validade (portão); nunca entra na escolha
    scen_info = {"construction": cset.summary(), "test": tset.summary()}
    n_c, n_t = len(cset.valid), len(tset.valid)
    if n_c < MIN_VALID_CONSTRUCTION or n_t < MIN_VALID_TEST:
        msg = (
            f"dados insuficientes: {n_c}/{cset.n_expected} pares válidos na construção "
            f"(mínimo {MIN_VALID_CONSTRUCTION}) e {n_t}/{tset.n_expected} no teste "
            f"(mínimo {MIN_VALID_TEST})"
        )
        return _empty_result(
            "DADOS_INSUFICIENTES",
            msg,
            scenarios=scen_info,
            n_plans=len(plans),
            x_hash=_x_hash(plans),
            **common,
        )

    ref, sbar = construction_climatology(cset)
    prm = params.with_reference(ref)
    pairs = [s.pair for s in cset.valid]
    comp = components_tensor(plans, pairs, prm)  # (n, S, 3)
    comp_bar = components_tensor(plans, [sbar], prm)[:, 0, :]  # (n, 3)
    ids = [p.id for p in plans]

    def choose(
        ww: Sequence[float],
    ) -> tuple[Selection, Selection, Selection, np.ndarray, np.ndarray]:
        L = loss_matrix(comp, ww)
        Lb = loss_matrix(comp_bar, ww)
        return select_conv(L, Lb, ids), select_exp(L, ids), select_mmr(L, ids), L, Lb

    s1, s1b, s2, L, Lbar = choose(w)
    policies: dict[str, Any] = {}
    picks: list[tuple[str, int]] = []
    p0_plan, p0_reasons = resolve_spec(catalog, soil, p0, plans)
    if p0_plan is None:
        policies["P0"] = {
            "status": "INVIAVEL_SOB_ZARC",
            "spec": p0.to_dict(),
            "reasons": p0_reasons,
            "message": "plano atual inviável sob ZARC <= 20%; sai da comparação de regret",
        }
    else:
        i0 = ids.index(p0_plan.id)
        policies["P0"] = {**_policy_entry(catalog, plans, i0, L, Lbar, None), "spec": p0.to_dict()}
        picks.append(("P0", i0))
    for name, sel in (("P1", s1), ("P1b", s1b), ("P2", s2)):
        policies[name] = _policy_entry(catalog, plans, sel.index, L, Lbar, sel)
        picks.append((name, sel.index))

    F = objective_vectors(comp)
    front, equivalents = pareto_front(F, ids)
    alts = select_alternatives(plans, F, front, picks, w, k_alternatives, min_hamming)

    sens = []
    use_fb = {"P1": 0, "P1b": 0, "P2": 0}
    for ww in WEIGHT_GRID:
        a1, a1b, a2, _, _ = choose(ww)
        row = {
            "weights": list(ww),
            "P1": list(ids[a1.index]),
            "P1b": list(ids[a1b.index]),
            "P2": list(ids[a2.index]),
            "same_plan": a1.index == a1b.index == a2.index,
            "id_tiebreak": {"P1": a1.id_tiebreak, "P1b": a1b.id_tiebreak, "P2": a2.id_tiebreak},
        }
        for name, a in (("P1", a1), ("P1b", a1b), ("P2", a2)):
            use_fb[name] += bool(plans[a.index].fallow_windows())
        sens.append(row)

    in_front_fb = sum(bool(plans[i].fallow_windows()) for i in front)
    fallow_usage = {
        "x_plans_with_fallow": fallow_summary(plans)["com_pousio"],
        "policies_w_star": {
            n: policies[n].get("fallow_windows", [])
            for n in POLICIES
            if n in policies and policies[n]["status"] == "OK"
        },
        "grid_selections_with_fallow": use_fb,
        "grid_size": len(WEIGHT_GRID),
        "front_with_fallow": in_front_fb,
        "front_size": len(front),
        "alternatives_with_fallow": sum(
            bool(plans[a["plan_index"]].fallow_windows()) for a in alts
        ),
    }
    obj_rows = [[list(plans[i].id), *F[i].tolist()] for i in range(len(plans))]
    pareto = {
        "front_ids": [list(ids[i]) for i in front],
        "equivalents": {
            ",".join(map(str, ids[r])): [list(ids[i]) for i in e]
            for r, e in equivalents.items()
            if e
        },
        "n_front": len(front),
        "objectives": ["mean_g1", "max_g1", "mean_g2", "mean_g3"],
    }
    alts_out = [{k: v for k, v in a.items() if k != "plan_index"} for a in alts]
    climatology = {
        "n_years": ref.n_years,
        "years": list(ref.labels),
        "precip_sha256": ref.sha256(),
        "sbar_sha256": hashlib.sha256(
            np.concatenate([sbar.P, sbar.TX, sbar.TN]).tobytes()
        ).hexdigest(),
    }
    return RunResult(
        status="OK",
        message="ok",
        n_plans=len(plans),
        x_hash=_x_hash(plans),
        fallow=fallow_summary(plans),
        rejections_summary=summarize_rejections(rejections),
        rejections=rej_dicts,
        diagnosis={},
        scenarios=scen_info,
        climatology=climatology,
        policies=policies,
        objectives=obj_rows,
        pareto=pareto,
        alternatives=alts_out,
        sensitivity=sens,
        fallow_usage=fallow_usage,
        protocol_version=PROTOCOL_VERSION,
        risk_limit_pct=ZARC_RISK_LIMIT_PCT,
        weight_grid=[list(g) for g in WEIGHT_GRID],
        snapshot_id=snapshot_id,
        config_hash=cfg_hash,
        soil=soil.describe(),
        weights=list(w),
    )


__all__ = [
    "DEFAULT_WEIGHTS",
    "HORIZON_DAYS",
    "POLICIES",
    "WEIGHT_GRID",
    "ClimatologyError",
    "OptimizationError",
    "RunResult",
    "Scenario",
    "ScenarioSet",
    "build_scenarios",
    "components_tensor",
    "construction_climatology",
    "loss_matrix",
    "objective_vectors",
    "pareto_front",
    "regret_matrix",
    "run",
    "select_alternatives",
    "select_conv",
    "select_exp",
    "select_mmr",
]
