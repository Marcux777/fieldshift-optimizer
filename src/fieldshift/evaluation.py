# AI-assisted (OpenBot agent)
"""Avaliador retrospectivo independente (protocolo v1.1, docs/EVALUATION_PROTOCOL.md).

Dono: Evaluation & Benchmark Engineer. Núcleo puro: sem I/O, rede ou Streamlit.

Princípios:
- O avaliador recebe planos FIXADOS por política e RECALCULA as perdas a partir do plano e do
  clima, via um ``LossModel`` (adaptador sobre ``indicators.loss_components``). Nunca aceita
  escores prontos do otimizador.
- Parâmetros dependentes de dados (medianas de e3), o clima médio s̄ e o P25 seco vêm SÓ dos
  cenários de construção. Os cenários de teste entram só para medir.
- As regras de seleção de referência (MMR, CONV, EXP) são reimplementadas aqui a partir do
  protocolo, para auditar a escolha do otimizador e para a grade de sensibilidade.
"""

from __future__ import annotations

import hashlib
import itertools
import json
import math
from collections.abc import Callable, Hashable, Mapping, Sequence
from dataclasses import dataclass, field
from typing import Any, Protocol

import numpy as np
import pandas as pd

PROTOCOL_VERSION = "1.1"
PROTOCOL_SHA256 = "ee8e51c5b2eecc7319818a5645ef15b84d677eabab35caad1a7e9fc41deaefb9"

CLIMATE_VARS = ("PRECTOTCORR", "T2M_MAX", "T2M_MIN")  # séries usadas nos indicadores
# Validade do cenário (protocolo v1.1 §1.3): as quatro variáveis completas, como no otimizador.
VALIDITY_VARS = ("PRECTOTCORR", "T2M", "T2M_MAX", "T2M_MIN")
DAYS_PER_YEAR = 365
HORIZON_DAYS = 730

OUTCOME_SUSTAINED = "SUSTENTADO"
OUTCOME_NEGATIVE = "NEGATIVO"
OUTCOME_INCONCLUSIVE = "INCONCLUSIVO"
OUTCOME_INFEASIBLE = "INVIAVEL"
OUTCOME_INSUFFICIENT = "DADOS_INSUFICIENTES"

POLICIES = ("P0", "P1", "P1b", "P2")


class EvaluationError(RuntimeError):
    """Falha que impede a avaliação (gate M0, s̄ incompleto, vazamento detectado)."""


def weight_grid() -> list[tuple[float, float, float]]:
    """15 triplas (a,b,c)/4 com a+b+c = 4; ordem decrescente de exposição, depois cobertura."""
    grid = [(a / 4, b / 4, (4 - a - b) / 4) for a in range(4, -1, -1) for b in range(4 - a, -1, -1)]
    return grid


DEFAULT_WEIGHTS = (0.50, 0.25, 0.25)


@dataclass(frozen=True)
class EvalProtocol:
    construction_first_start: int = 1981  # par (1981/82, 1982/83)
    construction_last_start: int = 2008  # par (2008/09, 2009/10)
    test_first_start: int = 2010  # par (2010/11, 2011/12)
    test_last_start: int = 2023  # par (2023/24, 2024/25)
    boundary_date: str = "2010-09-01"
    epsilon: float = 0.02
    rel_gain: float = 0.10
    n_boot: int = 10_000
    block_size: int = 2
    seed: int = 20261114
    min_construction_pairs: int = 20
    min_test_pairs: int = 10
    decimals: int = 9
    default_weights: tuple[float, float, float] = DEFAULT_WEIGHTS
    dry_months: tuple[int, ...] = (10, 11, 12, 1, 2, 3)
    dry_quantile: float = 0.25
    min_dry_pairs_inferential: int = 3
    version: str = PROTOCOL_VERSION
    sha256: str = PROTOCOL_SHA256

    def to_dict(self) -> dict[str, Any]:
        d = {k: getattr(self, k) for k in self.__dataclass_fields__}
        d["default_weights"] = list(self.default_weights)
        d["dry_months"] = list(self.dry_months)
        return d

    def config_hash(self) -> str:
        return sha256_json(self.to_dict())


# ---------------------------------------------------------------------------
# Cenários
# ---------------------------------------------------------------------------


@dataclass(frozen=True)
class AgriYear:
    """Um ano agrícola (1/set de y a 31/ago de y+1), 365 dias, 29/fev descartado."""

    start: int
    dates: tuple[str, str]  # primeira e última data
    precip: np.ndarray
    tmax: np.ndarray
    tmin: np.ndarray
    n_missing: dict[str, int]

    @property
    def label(self) -> str:
        return f"{self.start}/{(self.start + 1) % 100:02d}"

    @property
    def valid(self) -> bool:
        return len(self.precip) == DAYS_PER_YEAR and sum(self.n_missing.values()) == 0


@dataclass(frozen=True)
class Scenario:
    """Par de anos agrícolas consecutivos (y, y+1): 730 dias de P, TX, TN."""

    start: int
    label: str
    precip: np.ndarray
    tmax: np.ndarray
    tmin: np.ndarray
    first_date: str
    last_date: str
    years: tuple[int, ...]
    valid: bool = True
    synthetic: bool = False  # True só para o clima médio s̄


def _agri_year(climate: pd.DataFrame, y: int) -> AgriYear:
    start, end = pd.Timestamp(f"{y}-09-01"), pd.Timestamp(f"{y + 1}-08-31")
    full = pd.date_range(start, end, freq="D")
    full = full[~((full.month == 2) & (full.day == 29))]
    sub = climate.reindex(full)  # dias faltando no índice viram NaN (ausente, nunca 0)
    arrays = {v: sub[v].to_numpy(dtype=float) for v in VALIDITY_VARS}
    missing = {v: int(np.isnan(a).sum()) for v, a in arrays.items()}
    return AgriYear(
        start=y,
        dates=(str(full[0].date()), str(full[-1].date())),
        precip=arrays["PRECTOTCORR"],
        tmax=arrays["T2M_MAX"],
        tmin=arrays["T2M_MIN"],
        n_missing=missing,
    )


def build_agri_years(climate: pd.DataFrame, first: int, last: int) -> dict[int, AgriYear]:
    """Anos agrícolas de ``first`` a ``last`` (inclusive). -999 vira ausente antes de tudo."""
    missing_cols = [v for v in VALIDITY_VARS if v not in climate.columns]
    if missing_cols:
        raise EvaluationError(f"clima sem colunas obrigatórias: {missing_cols}")
    clim = climate[list(VALIDITY_VARS)].astype(float).replace(-999.0, np.nan)
    clim.index = pd.DatetimeIndex(clim.index)
    if clim.index.has_duplicates:
        raise EvaluationError("clima com datas duplicadas")
    return {y: _agri_year(clim, y) for y in range(first, last + 1)}


def pair_scenario(a: AgriYear, b: AgriYear) -> Scenario:
    return Scenario(
        start=a.start,
        label=f"{a.label}–{b.label}",
        precip=np.concatenate([a.precip, b.precip]),
        tmax=np.concatenate([a.tmax, b.tmax]),
        tmin=np.concatenate([a.tmin, b.tmin]),
        first_date=a.dates[0],
        last_date=b.dates[1],
        years=(a.start, b.start),
        valid=a.valid and b.valid,
    )


@dataclass(frozen=True)
class ScenarioSplit:
    construction: tuple[Scenario, ...]  # só válidos, ordem cronológica
    test: tuple[Scenario, ...]
    excluded: tuple[dict[str, Any], ...]  # cenários inválidos com motivo
    construction_years: tuple[AgriYear, ...]  # anos válidos usados por s̄ e P25


def split_scenarios(climate: pd.DataFrame, protocol: EvalProtocol) -> ScenarioSplit:
    """Monta os pares de construção e teste. Nenhum par cruza a fronteira 2009/10→2010/11."""
    years = build_agri_years(
        climate, protocol.construction_first_start, protocol.test_last_start + 1
    )
    excluded: list[dict[str, Any]] = []

    def pairs(first: int, last: int, role: str) -> list[Scenario]:
        out = []
        for y in range(first, last + 1):
            sc = pair_scenario(years[y], years[y + 1])
            if sc.valid:
                out.append(sc)
            else:
                miss = {
                    f"{yr.label}:{v}": n
                    for yr in (years[y], years[y + 1])
                    for v, n in yr.n_missing.items()
                    if n
                }
                excluded.append({"role": role, "scenario": sc.label, "missing": miss})
        return out

    cons = pairs(protocol.construction_first_start, protocol.construction_last_start, "construcao")
    test = pairs(protocol.test_first_start, protocol.test_last_start, "teste")
    boundary = pd.Timestamp(protocol.boundary_date)
    for sc in cons:
        if pd.Timestamp(sc.last_date) >= boundary:
            raise EvaluationError(
                f"vazamento: par de construção {sc.label} cruza {boundary.date()}"
            )
    for sc in test:
        if pd.Timestamp(sc.first_date) < boundary:
            raise EvaluationError(f"par de teste {sc.label} começa antes de {boundary.date()}")
    cons_year_ids = sorted({y for sc in cons for y in sc.years})
    return ScenarioSplit(
        construction=tuple(cons),
        test=tuple(test),
        excluded=tuple(excluded),
        construction_years=tuple(years[y] for y in cons_year_ids),
    )


def mean_climate(construction_years: Sequence[AgriYear]) -> Scenario:
    """s̄: média diária por (mês, dia) dos anos agrícolas válidos da construção, aplicada aos 2 anos.

    Sem imputação: um dia sem nenhum valor aborta a execução.
    """
    if not construction_years:
        raise EvaluationError("s̄ não calculável: nenhum ano de construção válido")
    stack = {
        "precip": np.vstack([y.precip for y in construction_years]),
        "tmax": np.vstack([y.tmax for y in construction_years]),
        "tmin": np.vstack([y.tmin for y in construction_years]),
    }
    means = {}
    for k, arr in stack.items():
        counts = (~np.isnan(arr)).sum(axis=0)
        if (counts == 0).any():
            raise EvaluationError(f"s̄ não calculável: dia sem valor em {k}")
        means[k] = np.nanmean(arr, axis=0)
    return Scenario(
        start=-1,
        label="clima_medio_construcao",
        precip=np.concatenate([means["precip"]] * 2),
        tmax=np.concatenate([means["tmax"]] * 2),
        tmin=np.concatenate([means["tmin"]] * 2),
        first_date="",
        last_date="",
        years=tuple(y.start for y in construction_years),
        synthetic=True,
    )


def dry_threshold(construction_years: Sequence[AgriYear], protocol: EvalProtocol) -> float:
    """P25 da chuva out–mar nos anos agrícolas de construção (só construção)."""
    totals = [_dry_season_total(y, protocol) for y in construction_years]
    return float(np.quantile(totals, protocol.dry_quantile))


def _dry_season_total(year: AgriYear, protocol: EvalProtocol) -> float:
    dates = pd.date_range(f"{year.start}-09-01", f"{year.start + 1}-08-31", freq="D")
    dates = dates[~((dates.month == 2) & (dates.day == 29))]
    mask = np.isin(dates.month, protocol.dry_months)
    return float(np.sum(year.precip[mask]))


def scenario_dry_season_totals(sc: Scenario, protocol: EvalProtocol) -> tuple[float, float]:
    out = []
    for i, y in enumerate(sc.years):
        ay = AgriYear(
            start=y,
            dates=("", ""),
            precip=sc.precip[i * DAYS_PER_YEAR : (i + 1) * DAYS_PER_YEAR],
            tmax=sc.tmax[i * DAYS_PER_YEAR : (i + 1) * DAYS_PER_YEAR],
            tmin=sc.tmin[i * DAYS_PER_YEAR : (i + 1) * DAYS_PER_YEAR],
            n_missing={},
        )
        out.append(_dry_season_total(ay, protocol))
    return out[0], out[1]


# ---------------------------------------------------------------------------
# Modelo de perda (adaptador) e matriz de componentes
# ---------------------------------------------------------------------------


class LossModel(Protocol):
    """Adaptador sobre ``indicators``. ``fit`` só pode ver cenários de construção."""

    def fit(self, construction: Sequence[Scenario]) -> Any: ...

    def components(
        self, plan: Any, scenario: Scenario, params: Any
    ) -> tuple[float, float, float]: ...


def plan_key(plan: Any) -> tuple:
    pid = plan.id if hasattr(plan, "id") else plan
    return tuple(pid) if isinstance(pid, (tuple, list)) else (pid,)


def component_matrix(
    plans: Sequence[Any], scenarios: Sequence[Scenario], loss_model: LossModel, params: Any
) -> np.ndarray:
    """G[i, j, k] = componente k do plano i no cenário j, RECALCULADO do plano e do clima."""
    g = np.empty((len(plans), len(scenarios), 3), dtype=float)
    for i, p in enumerate(plans):
        for j, sc in enumerate(scenarios):
            comps = loss_model.components(p, sc, params)
            g[i, j, :] = comps
    if not np.isfinite(g).all():
        raise EvaluationError("componente não finito (NaN/inf) na matriz de perdas")
    if (g < -1e-12).any() or (g > 1 + 1e-12).any():
        raise EvaluationError("componente fora de [0,1]")
    return g


def q(x: Any, decimals: int = 9) -> Any:
    return np.round(x, decimals)


def losses(g: np.ndarray, w: Sequence[float]) -> np.ndarray:
    return g[..., 0] * w[0] + g[..., 1] * w[1] + g[..., 2] * w[2]


def regret(L: np.ndarray) -> np.ndarray:
    """R(x,s) = L(x,s) − min_x′ L(x′,s); L tem forma (planos, cenários)."""
    return L - L.min(axis=0, keepdims=True)


# ---------------------------------------------------------------------------
# Seleção de referência (protocolo item 1; desempates da Optimization R5)
# ---------------------------------------------------------------------------


def _argmin_lex(keys: list[tuple]) -> int:
    return min(range(len(keys)), key=lambda i: keys[i])


def select_mmr(L_cons: np.ndarray, ids: Sequence[tuple], decimals: int = 9) -> int:
    R = regret(L_cons)
    mx, mn = q(R.max(axis=1), decimals), q(R.mean(axis=1), decimals)
    return _argmin_lex([(mx[i], mn[i], ids[i]) for i in range(len(ids))])


def select_conv(
    L_bar: np.ndarray, L_cons: np.ndarray, ids: Sequence[tuple], decimals: int = 9
) -> int:
    lb, mn = q(L_bar, decimals), q(L_cons.mean(axis=1), decimals)
    return _argmin_lex([(lb[i], mn[i], ids[i]) for i in range(len(ids))])


def select_exp(L_cons: np.ndarray, ids: Sequence[tuple], decimals: int = 9) -> int:
    R = regret(L_cons)
    mn, mx = q(L_cons.mean(axis=1), decimals), q(R.max(axis=1), decimals)
    return _argmin_lex([(mn[i], mx[i], ids[i]) for i in range(len(ids))])


def reference_selection(
    L_cons: np.ndarray, L_bar: np.ndarray, ids: Sequence[tuple], decimals: int = 9
) -> dict[str, int]:
    return {
        "P1": select_conv(L_bar, L_cons, ids, decimals),
        "P1b": select_exp(L_cons, ids, decimals),
        "P2": select_mmr(L_cons, ids, decimals),
    }


# ---------------------------------------------------------------------------
# Estatística
# ---------------------------------------------------------------------------


def block_bootstrap_ci(
    d: np.ndarray, block_size: int, n_boot: int, seed: int, stat: Callable = np.median
) -> tuple[float, float]:
    """IC 95% percentil por bootstrap em blocos não sobrepostos de pares consecutivos."""
    n = len(d)
    if n == 0:
        return (math.nan, math.nan)
    blocks = [d[i : i + block_size] for i in range(0, n, block_size)]
    nb = len(blocks)
    mat = np.full((nb, block_size), np.nan)
    for i, b in enumerate(blocks):
        mat[i, : len(b)] = b
    rng = np.random.default_rng(seed)
    idx = rng.integers(0, nb, size=(n_boot, nb))
    samples = mat[idx].reshape(n_boot, nb * block_size)
    reducer = np.nanmedian if stat is np.median else np.nanmean
    vals = reducer(samples, axis=1)
    lo, hi = np.quantile(vals, [0.025, 0.975])
    return float(lo), float(hi)


def _avg_ranks(x: np.ndarray) -> np.ndarray:
    order = np.argsort(x, kind="mergesort")
    ranks = np.empty(len(x))
    sx = x[order]
    i = 0
    while i < len(x):
        j = i
        while j + 1 < len(x) and sx[j + 1] == sx[i]:
            j += 1
        ranks[order[i : j + 1]] = (i + j) / 2 + 1
        i = j + 1
    return ranks


def wilcoxon_exact(d: np.ndarray, decimals: int = 9) -> dict[str, Any]:
    """Wilcoxon pareado exato bilateral (sinais por enumeração; zeros descartados)."""
    d = q(np.asarray(d, dtype=float), decimals)
    nz = d[d != 0]
    n = len(nz)
    if n == 0:
        return {"n_nonzero": 0, "W_plus": 0.0, "p_two_sided": 1.0}
    ranks = _avg_ranks(np.abs(nz))
    w_obs = float(ranks[nz > 0].sum())
    center = ranks.sum() / 2
    dev_obs = abs(w_obs - center)
    count = 0
    total = 0
    for signs in itertools.product((0, 1), repeat=n):
        w = float(np.dot(signs, ranks))
        total += 1
        if abs(w - center) >= dev_obs - 1e-12:
            count += 1
    return {"n_nonzero": n, "W_plus": w_obs, "p_two_sided": count / total}


# ---------------------------------------------------------------------------
# Relatório
# ---------------------------------------------------------------------------


@dataclass
class PolicyMetrics:
    policy: str
    plan_id: list | None
    feasible: bool
    M1_max_regret: float | None = None
    M2_mean_regret: float | None = None
    M2_median_regret: float | None = None
    M3_dry_mean_loss: float | None = None
    M3_dry_mean_regret: float | None = None
    M4a_frac_regret_le_eps: float | None = None
    M4b_iqr_loss: float | None = None
    M4c_loo_mean_hamming: float | None = None
    mean_loss_test: float | None = None
    loss_mean_climate: float | None = None
    per_scenario_regret: list[float] = field(default_factory=list)
    per_scenario_loss: list[float] = field(default_factory=list)


@dataclass
class Comparison:
    a: str
    b: str
    identical_plans: bool
    n: int
    wins: int
    ties: int
    losses: int
    median_D: float | None
    ci95_median_D: tuple[float, float] | None
    mean_D: float | None
    ci95_mean_D: tuple[float, float] | None
    wilcoxon_nonoverlap: dict[str, Any] | None
    m1_a: float | None
    m1_b: float | None
    classification: str
    reason: str


@dataclass
class EvalReport:
    outcome: str
    headline: str
    protocol: dict[str, Any]
    protocol_config_hash: str
    weights: list[float]
    n_plans: int
    construction: list[str]
    test: list[str]
    excluded_scenarios: list[dict[str, Any]]
    params_hash: str | None = None
    mean_climate_hash: str | None = None
    dry_threshold_mm: float | None = None
    dry_test_scenarios: list[str] = field(default_factory=list)
    M0_feasibility: float | None = None
    p0_status: str = "nao_informado"
    policies: dict[str, PolicyMetrics] = field(default_factory=dict)
    M5_price_of_robustness: float | None = None
    comparisons: list[Comparison] = field(default_factory=list)
    audit: dict[str, Any] = field(default_factory=dict)
    sensitivity: list[dict[str, Any]] = field(default_factory=list)
    limitations: list[str] = field(default_factory=list)
    diagnostics: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return _jsonable(
            {
                "outcome": self.outcome,
                "headline": self.headline,
                "protocol": self.protocol,
                "protocol_config_hash": self.protocol_config_hash,
                "weights": self.weights,
                "n_plans": self.n_plans,
                "construction": self.construction,
                "test": self.test,
                "excluded_scenarios": self.excluded_scenarios,
                "params_hash": self.params_hash,
                "mean_climate_hash": self.mean_climate_hash,
                "dry_threshold_mm": self.dry_threshold_mm,
                "dry_test_scenarios": self.dry_test_scenarios,
                "M0_feasibility": self.M0_feasibility,
                "p0_status": self.p0_status,
                "policies": {k: v.__dict__ for k, v in self.policies.items()},
                "M5_price_of_robustness": self.M5_price_of_robustness,
                "comparisons": [c.__dict__ for c in self.comparisons],
                "audit": self.audit,
                "sensitivity": self.sensitivity,
                "limitations": self.limitations,
                "diagnostics": self.diagnostics,
            }
        )

    def result_hash(self) -> str:
        return sha256_json(self.to_dict())

    def to_markdown(self) -> str:
        return render_markdown(self)


def _jsonable(x: Any) -> Any:
    if isinstance(x, dict):
        return {str(k): _jsonable(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [_jsonable(v) for v in x]
    if isinstance(x, (np.floating, float)):
        f = float(x)
        return None if math.isnan(f) else round(f, 12)
    if isinstance(x, (np.integer,)):
        return int(x)
    if isinstance(x, np.bool_):
        return bool(x)
    if isinstance(x, np.ndarray):
        return _jsonable(x.tolist())
    return x


def sha256_json(obj: Any) -> str:
    blob = json.dumps(_jsonable(obj), sort_keys=True, ensure_ascii=False, separators=(",", ":"))
    return hashlib.sha256(blob.encode("utf-8")).hexdigest()


def _array_hash(*arrays: np.ndarray) -> str:
    h = hashlib.sha256()
    for a in arrays:
        h.update(np.ascontiguousarray(np.round(a, 9)).tobytes())
    return h.hexdigest()


# ---------------------------------------------------------------------------
# Classificação
# ---------------------------------------------------------------------------


def classify(
    *,
    identical: bool,
    ci: tuple[float, float] | None,
    m1_a: float,
    m1_b: float,
    m0_ok: bool,
    protocol: EvalProtocol,
) -> tuple[str, str]:
    """Regras do item 3 do protocolo; ``a`` = P2, ``b`` = comparador."""
    if identical:
        return OUTCOME_INCONCLUSIVE, "planos idênticos: a robustez não mudou a decisão"
    if ci is None or any(math.isnan(v) for v in ci):
        return OUTCOME_INCONCLUSIVE, "IC indisponível"
    lo, hi = ci
    eps = protocol.epsilon
    reduction = m1_b - m1_a
    rel_ok = m1_b > 0 and reduction >= protocol.rel_gain * m1_b
    if hi < 0 and rel_ok and reduction >= eps and m0_ok:
        return OUTCOME_SUSTAINED, "IC 95% < 0 e M1 cai ≥10% e ≥ε"
    if lo > 0:
        return OUTCOME_NEGATIVE, "IC 95% inteiramente > 0"
    if q(m1_a) >= q(m1_b):
        return OUTCOME_NEGATIVE, "M1(P2) ≥ M1 do comparador"
    return OUTCOME_INCONCLUSIVE, "IC cruza 0 ou ganho de M1 abaixo do limiar"


def _compare(
    a: str,
    b: str,
    R_test: np.ndarray,
    idx: dict[str, int],
    ids: Sequence[tuple],
    nonoverlap_mask: np.ndarray,
    m0_ok: bool,
    protocol: EvalProtocol,
) -> Comparison:
    ra, rb = R_test[idx[a]], R_test[idx[b]]
    d = q(ra - rb, protocol.decimals)
    eps = protocol.epsilon
    identical = ids[idx[a]] == ids[idx[b]]
    ci_med = block_bootstrap_ci(d, protocol.block_size, protocol.n_boot, protocol.seed, np.median)
    ci_mean = block_bootstrap_ci(d, protocol.block_size, protocol.n_boot, protocol.seed, np.mean)
    m1a, m1b = float(ra.max()), float(rb.max())
    cls, reason = classify(
        identical=identical, ci=ci_med, m1_a=m1a, m1_b=m1b, m0_ok=m0_ok, protocol=protocol
    )
    return Comparison(
        a=a,
        b=b,
        identical_plans=identical,
        n=len(d),
        wins=int((d < -eps).sum()),
        ties=int((np.abs(d) <= eps).sum()),
        losses=int((d > eps).sum()),
        median_D=float(np.median(d)),
        ci95_median_D=ci_med,
        mean_D=float(np.mean(d)),
        ci95_mean_D=ci_mean,
        wilcoxon_nonoverlap=wilcoxon_exact(d[nonoverlap_mask], protocol.decimals),
        m1_a=m1a,
        m1_b=m1b,
        classification=cls,
        reason=reason,
    )


# ---------------------------------------------------------------------------
# Avaliação
# ---------------------------------------------------------------------------

STANDARD_LIMITATIONS = [
    "Janelas ZARC calibradas com anos que se sobrepõem ao teste: restrição fixa e igual para "
    "todas as políticas (vazamento residual declarado, não removível).",
    "PRECTOTCORR pode ter correção de viés baseada em climatologia longa (vazamento residual "
    "declarado, não verificado).",
    "Uma célula NASA POWER (-18,0/-50,625); célula vizinha não avaliada nesta execução.",
    "Indicadores de exposição e de calendário; não medem produtividade, lucro, água nem solo.",
    "M4c (leave-one-pair-out) mantém fixas as medianas de e3 da construção completa "
    "(simplificação declarada).",
    "Bootstrap com 7 blocos e Wilcoxon com 7 pares: poder estatístico baixo.",
]


def _genes(plan: Any) -> tuple:
    """Um gene por janela (cultura, semeadura) quando o plano expõe ocupações; senão o ID."""
    occ = getattr(plan, "occupations", None)
    if occ is not None:
        return tuple((o.crop, o.sow_t) for o in occ)
    return plan_key(plan)


def _hamming(a: tuple, b: tuple) -> int:
    if len(a) != len(b):
        return max(len(a), len(b))
    return sum(x != y for x, y in zip(a, b, strict=True))


def evaluate_plans(
    *,
    plans: Sequence[Any],
    policy_plans: Mapping[str, Any | None],
    climate: pd.DataFrame,
    loss_model: LossModel,
    protocol: EvalProtocol | None = None,
    weights: Sequence[float] | None = None,
    p0_status: str | None = None,
    check_plan: Callable[[Any], list[str]] | None = None,
    run_grid: bool = True,
    optimizer_params_hash: str | None = None,
) -> EvalReport:
    """Avalia planos fixados por política, recalculando todas as perdas.

    ``plans`` é o conjunto viável X enumerado (necessário para o oráculo do regret).
    ``policy_plans`` mapeia P0/P1/P1b/P2 ao plano escolhido pelo otimizador (P0 pode ser None).
    """
    protocol = protocol or EvalProtocol()
    w = tuple(weights or protocol.default_weights)
    split = split_scenarios(climate, protocol)
    report = EvalReport(
        outcome="",
        headline="",
        protocol=protocol.to_dict(),
        protocol_config_hash=protocol.config_hash(),
        weights=list(w),
        n_plans=len(plans),
        construction=[s.label for s in split.construction],
        test=[s.label for s in split.test],
        excluded_scenarios=list(split.excluded),
        limitations=list(STANDARD_LIMITATIONS),
    )

    if len(plans) == 0:
        report.outcome = OUTCOME_INFEASIBLE
        report.headline = (
            "INVIAVEL: nenhum plano viável sob ZARC ≤ 20% no solo informado; "
            "a avaliação não roda (limite não relaxado)."
        )
        return report
    if (
        len(split.construction) < protocol.min_construction_pairs
        or len(split.test) < protocol.min_test_pairs
    ):
        report.outcome = OUTCOME_INSUFFICIENT
        report.headline = (
            f"DADOS_INSUFICIENTES: {len(split.construction)} pares de construção válidos "
            f"(mín. {protocol.min_construction_pairs}) e {len(split.test)} de teste "
            f"(mín. {protocol.min_test_pairs}); sem veredito."
        )
        return report

    ids = [plan_key(p) for p in plans]
    if len(set(ids)) != len(ids):
        raise EvaluationError("IDs de plano duplicados em X")
    id_to_idx = {pid: i for i, pid in enumerate(ids)}

    # Parâmetros e referências SÓ com construção.
    params = loss_model.fit(split.construction)
    s_bar = mean_climate(split.construction_years)
    p25 = dry_threshold(split.construction_years, protocol)
    fp = getattr(loss_model, "fingerprint", None)
    report.params_hash = (
        fp(params) if fp else (sha256_json(params) if _is_jsonable(params) else None)
    )
    report.mean_climate_hash = _array_hash(s_bar.precip, s_bar.tmax, s_bar.tmin)
    report.dry_threshold_mm = p25
    if optimizer_params_hash is not None:
        report.audit["optimizer_params_hash"] = optimizer_params_hash
        report.audit["params_hash_match"] = optimizer_params_hash == report.params_hash

    # Perdas recalculadas: construção, s̄, teste.
    G_cons = component_matrix(plans, split.construction, loss_model, params)
    G_bar = component_matrix(plans, [s_bar], loss_model, params)[:, 0, :]
    G_test = component_matrix(plans, split.test, loss_model, params)

    # Políticas: o avaliador só aceita planos que estão em X (M0) e passa pelo verificador.
    pol_idx: dict[str, int] = {}
    m0_checked, m0_pass = 0, 0
    violations: dict[str, list[str]] = {}
    for pol in ("P1", "P1b", "P2"):
        plan = policy_plans.get(pol)
        if plan is None:
            raise EvaluationError(f"política {pol} sem plano")
        m0_checked += 1
        key = plan_key(plan)
        v = [] if key in id_to_idx else ["plano fora de X (viola ZARC ≤ 20% ou calendário)"]
        if check_plan is not None:
            v += list(check_plan(plan))
        if v:
            violations[pol] = v
        else:
            m0_pass += 1
            pol_idx[pol] = id_to_idx[key]
    report.M0_feasibility = m0_pass / m0_checked
    if violations:
        report.audit["M0_violations"] = violations
        raise EvaluationError(f"gate M0 < 100%: {violations}")

    p0 = policy_plans.get("P0")
    if p0 is None:
        report.p0_status = p0_status or "nao_informado"
    else:
        key0 = plan_key(p0)
        v0 = list(check_plan(p0)) if check_plan is not None else []
        if key0 in id_to_idx and not v0:
            pol_idx["P0"] = id_to_idx[key0]
            report.p0_status = "viavel"
        else:
            report.p0_status = "inviavel_sob_ZARC_20"
            report.audit["P0_violations"] = v0 or ["plano fora de X"]

    # Auditoria: a escolha do otimizador bate com a regra do protocolo?
    L_cons, L_bar, L_test = losses(G_cons, w), losses(G_bar, w), losses(G_test, w)
    ref = reference_selection(L_cons, L_bar, ids, protocol.decimals)
    report.audit["reference_selection"] = {k: list(ids[v]) for k, v in ref.items()}
    report.audit["optimizer_matches_reference"] = {k: ids[ref[k]] == ids[pol_idx[k]] for k in ref}

    R_test = regret(L_test)
    R_cons = regret(L_cons)
    eps = protocol.epsilon

    dry_flags = []
    for sc in split.test:
        a, b = scenario_dry_season_totals(sc, protocol)
        dry_flags.append(a < p25 or b < p25)
    dry_mask = np.array(dry_flags, dtype=bool)
    report.dry_test_scenarios = [s.label for s, f in zip(split.test, dry_flags, strict=True) if f]
    nonoverlap = np.array(
        [(s.start - protocol.test_first_start) % 2 == 0 for s in split.test], dtype=bool
    )

    # M4c: leave-one-pair-out na construção (s̄ refeito; medianas de e3 fixas).
    loo = _leave_one_out(plans, split, loss_model, params, G_cons, w, ids, protocol, pol_idx)

    for pol in POLICIES:
        if pol not in pol_idx:
            if pol == "P0":
                report.policies[pol] = PolicyMetrics(
                    policy=pol,
                    plan_id=list(plan_key(p0)) if p0 is not None else None,
                    feasible=False,
                )
            continue
        i = pol_idx[pol]
        r, lt = R_test[i], L_test[i]
        pm = PolicyMetrics(
            policy=pol,
            plan_id=list(ids[i]),
            feasible=True,
            M1_max_regret=float(r.max()),
            M2_mean_regret=float(r.mean()),
            M2_median_regret=float(np.median(r)),
            M3_dry_mean_loss=float(lt[dry_mask].mean()) if dry_mask.any() else None,
            M3_dry_mean_regret=float(r[dry_mask].mean()) if dry_mask.any() else None,
            M4a_frac_regret_le_eps=float((r <= eps).mean()),
            M4b_iqr_loss=float(np.quantile(lt, 0.75) - np.quantile(lt, 0.25)),
            M4c_loo_mean_hamming=loo.get(pol),
            mean_loss_test=float(lt.mean()),
            loss_mean_climate=float(L_bar[i]),
            per_scenario_regret=[float(v) for v in r],
            per_scenario_loss=[float(v) for v in lt],
        )
        report.policies[pol] = pm

    report.M5_price_of_robustness = float(L_bar[pol_idx["P2"]] - L_bar[pol_idx["P1"]])
    report.diagnostics["construction_max_regret_by_policy"] = {
        k: float(R_cons[v].max()) for k, v in pol_idx.items()
    }
    report.diagnostics["n_dry_test"] = int(dry_mask.sum())
    report.diagnostics["M3_inferential"] = int(dry_mask.sum()) >= protocol.min_dry_pairs_inferential

    m0_ok = report.M0_feasibility == 1.0
    for b in ("P1b", "P1"):
        report.comparisons.append(
            _compare("P2", b, R_test, pol_idx, ids, nonoverlap, m0_ok, protocol)
        )

    primary = report.comparisons[0]
    secondary = report.comparisons[1]
    report.outcome = primary.classification
    report.headline = (
        f"{primary.classification}: P2 (minimax regret) vs P1b (perda esperada) no teste "
        f"{split.test[0].label.split('–')[0]}–{split.test[-1].label.split('–')[1]}; "
        f"mediana D = {primary.median_D:+.4f}, IC 95% [{primary.ci95_median_D[0]:+.4f}, "
        f"{primary.ci95_median_D[1]:+.4f}], M1 {primary.m1_a:.4f} vs {primary.m1_b:.4f}. "
        f"P2 vs P1 (clima médio): {secondary.classification}."
    )

    if run_grid:
        report.sensitivity = _grid(G_cons, G_bar, G_test, ids, nonoverlap, protocol)
    return report


def _is_jsonable(x: Any) -> bool:
    try:
        json.dumps(_jsonable(x))
        return True
    except TypeError:
        return False


def _leave_one_out(plans, split, loss_model, params, G_cons, w, ids, protocol, pol_idx):
    n = len(split.construction)
    full_bar_cache: dict[tuple, np.ndarray] = {}
    hams: dict[str, list[int]] = {"P1": [], "P1b": [], "P2": []}
    years_by_start = {y.start: y for y in split.construction_years}
    for k in range(n):
        keep = [j for j in range(n) if j != k]
        Lc = losses(G_cons[:, keep, :], w)
        yrs = tuple(sorted({y for j in keep for y in split.construction[j].years}))
        if yrs not in full_bar_cache:
            sb = mean_climate([years_by_start[y] for y in yrs])
            full_bar_cache[yrs] = losses(
                component_matrix(plans, [sb], loss_model, params)[:, 0, :], w
            )
        sel = reference_selection(Lc, full_bar_cache[yrs], ids, protocol.decimals)
        for pol in hams:
            hams[pol].append(_hamming(_genes(plans[sel[pol]]), _genes(plans[pol_idx[pol]])))
    return {pol: float(np.mean(v)) for pol, v in hams.items()}


def _grid(G_cons, G_bar, G_test, ids, nonoverlap, protocol):
    rows = []
    for gi, w in enumerate(weight_grid(), start=1):
        Lc, Lb, Lt = losses(G_cons, w), losses(G_bar, w), losses(G_test, w)
        sel = reference_selection(Lc, Lb, ids, protocol.decimals)
        Rt = regret(Lt)
        row: dict[str, Any] = {
            "grid_index": gi,
            "weights": list(w),
            "is_default": tuple(w) == tuple(protocol.default_weights),
            "plans": {k: list(ids[v]) for k, v in sel.items()},
            "M1": {k: float(Rt[v].max()) for k, v in sel.items()},
            "M2_mean": {k: float(Rt[v].mean()) for k, v in sel.items()},
        }
        for b in ("P1b", "P1"):
            c = _compare("P2", b, Rt, sel, ids, nonoverlap, True, protocol)
            row[f"P2_vs_{b}"] = {
                "classification": c.classification,
                "median_D": c.median_D,
                "ci95": c.ci95_median_D,
                "identical": c.identical_plans,
            }
        rows.append(row)
    return rows


# ---------------------------------------------------------------------------
# Renderização (mesmo formato e destaque para qualquer desfecho)
# ---------------------------------------------------------------------------


def _fmt(v: Any, nd: int = 4) -> str:
    if v is None:
        return "—"
    if isinstance(v, float):
        return f"{v:.{nd}f}"
    return str(v)


def render_markdown(r: EvalReport) -> str:
    lines = [f"**DESFECHO: {r.outcome}** — {r.headline}", ""]
    lines += [
        f"Protocolo v{r.protocol['version']} (SHA-256 {r.protocol['sha256']}); "
        f"config do protocolo {r.protocol_config_hash}.",
        f"Pesos (exposição, cobertura, diversidade) = {r.weights}. |X| = {r.n_plans}. "
        f"P0: {r.p0_status}. M0 = {_fmt(r.M0_feasibility, 2)}.",
        f"Construção: {len(r.construction)} pares; teste: {len(r.test)} pares; "
        f"excluídos: {len(r.excluded_scenarios)}.",
        "",
    ]
    if r.policies:
        lines += [
            "| Política | Plano | M1 regret máx | M2 média | M2 mediana | M3 L seco | M3 R seco "
            "| M4a R≤ε | M4b IQR L | M4c LOO Hamming | L(s̄) |",
            "|---|---|---|---|---|---|---|---|---|---|---|",
        ]
        for pol in POLICIES:
            pm = r.policies.get(pol)
            if pm is None:
                continue
            if not pm.feasible:
                lines.append(
                    f"| {pol} | {pm.plan_id} | inviável sob ZARC ≤ 20% (fora da comparação) "
                    "| | | | | | | | |"
                )
                continue
            lines.append(
                f"| {pol} | {pm.plan_id} | {_fmt(pm.M1_max_regret)} | {_fmt(pm.M2_mean_regret)} "
                f"| {_fmt(pm.M2_median_regret)} | {_fmt(pm.M3_dry_mean_loss)} "
                f"| {_fmt(pm.M3_dry_mean_regret)} | {_fmt(pm.M4a_frac_regret_le_eps, 2)} "
                f"| {_fmt(pm.M4b_iqr_loss)} | {_fmt(pm.M4c_loo_mean_hamming, 2)} "
                f"| {_fmt(pm.loss_mean_climate)} |"
            )
        lines += [
            "",
            f"M3: {r.diagnostics.get('n_dry_test')} pares secos no teste (P25 out–mar da "
            f"construção = {_fmt(r.dry_threshold_mm, 1)} mm)"
            + ("" if r.diagnostics.get("M3_inferential") else "; n < 3, só descritivo")
            + ".",
            f"M5 preço da robustez L(P2, s̄) − L(P1, s̄) = {_fmt(r.M5_price_of_robustness)}.",
            "",
            "| Comparação | Idênticos | V/E/D (ε) | Mediana D | IC 95% mediana | Média D "
            "| IC 95% média | Wilcoxon 7 pares (p) | Classe | Motivo |",
            "|---|---|---|---|---|---|---|---|---|---|",
        ]
        for c in r.comparisons:
            wx = c.wilcoxon_nonoverlap or {}
            lines.append(
                f"| {c.a} vs {c.b} | {c.identical_plans} | {c.wins}/{c.ties}/{c.losses} "
                f"| {_fmt(c.median_D)} | [{_fmt(c.ci95_median_D[0])}, {_fmt(c.ci95_median_D[1])}] "
                f"| {_fmt(c.mean_D)} | [{_fmt(c.ci95_mean_D[0])}, {_fmt(c.ci95_mean_D[1])}] "
                f"| {_fmt(wx.get('p_two_sided'))} (n≠0={wx.get('n_nonzero')}) "
                f"| **{c.classification}** | {c.reason} |"
            )
    if r.sensitivity:
        lines += [
            "",
            "Sensibilidade (grade de 15 pesos; só relatada, não muda o veredito em w*):",
            "",
            "| # | Pesos | P2 vs P1b | P2 vs P1 | M1 P1 / P1b / P2 |",
            "|---|---|---|---|---|",
        ]
        for row in r.sensitivity:
            m1 = row["M1"]
            lines.append(
                f"| {row['grid_index']}{' (padrão)' if row['is_default'] else ''} "
                f"| {row['weights']} | {row['P2_vs_P1b']['classification']} "
                f"| {row['P2_vs_P1']['classification']} "
                f"| {_fmt(m1['P1'])} / {_fmt(m1['P1b'])} / {_fmt(m1['P2'])} |"
            )
    if r.audit:
        lines += ["", "Auditoria: " + json.dumps(_jsonable(r.audit), ensure_ascii=False)]
    lines += ["", "Limitações:"] + [f"- {x}" for x in r.limitations]
    return "\n".join(lines) + "\n"


# ---------------------------------------------------------------------------
# Adaptador para o RunResult do otimizador (contrato docs/CONTRACT.md)
# ---------------------------------------------------------------------------


def evaluate(
    run_result: Any,
    climate: pd.DataFrame,
    protocol: EvalProtocol | None = None,
    *,
    plans: Sequence[Any],
    loss_model: LossModel,
    **kwargs: Any,
) -> EvalReport:
    """Contrato: recebe o RunResult, mas só usa os PLANOS escolhidos; ignora escores prontos."""
    policy_plans = _policy_plans_from_run(run_result, plans)
    kwargs.setdefault("p0_status", p0_status_from_run(run_result))
    return evaluate_plans(
        plans=plans,
        policy_plans=policy_plans,
        climate=climate,
        loss_model=loss_model,
        protocol=protocol,
        **kwargs,
    )


def _policy_plans_from_run(run_result: Any, plans: Sequence[Any]) -> dict[str, Any | None]:
    """Extrai SÓ o ID do plano de cada política; métricas e escores do otimizador são ignorados."""
    by_id: dict[Hashable, Any] = {plan_key(p): p for p in plans}
    raw = getattr(run_result, "policies", None)
    if raw is None and isinstance(run_result, Mapping):
        raw = run_result.get("policies")
    if raw is None:
        raise EvaluationError("RunResult sem 'policies'")
    out: dict[str, Any | None] = {}
    for pol in POLICIES:
        entry = raw.get(pol) if isinstance(raw, Mapping) else getattr(raw, pol, None)
        pid = None
        if isinstance(entry, Mapping):
            pid = entry.get("plan_id")
            if pid is None and isinstance(entry.get("plan"), Mapping):
                pid = entry["plan"].get("id")
        elif entry is not None:
            pid = getattr(entry, "plan_id", None)
        if pid is None:
            out[pol] = None
            continue
        key = tuple(pid) if isinstance(pid, (list, tuple)) else (pid,)
        out[pol] = by_id.get(key, _ForeignPlan(key))
    return out


def p0_status_from_run(run_result: Any) -> str | None:
    raw = getattr(run_result, "policies", None)
    if raw is None and isinstance(run_result, Mapping):
        raw = run_result.get("policies")
    entry = (raw or {}).get("P0") if isinstance(raw, Mapping) else None
    if isinstance(entry, Mapping) and entry.get("status") not in (None, "OK"):
        return "inviavel_sob_ZARC_20"
    return None


@dataclass(frozen=True)
class _ForeignPlan:
    """Plano citado pelo otimizador mas ausente de X (o avaliador o recusa)."""

    id: tuple


# ---------------------------------------------------------------------------
# Adaptador real sobre ``indicators`` (Optimization Engineer)
# ---------------------------------------------------------------------------


class IndicatorLossModel:
    """Recalcula (g1, g2, g3) com ``indicators.loss_components`` a partir do plano e do clima.

    A referência de e3 (``ConstructionClimatology``) é montada AQUI, só com os anos agrícolas dos
    pares válidos de construção que o avaliador recebeu, sem reaproveitar nada do otimizador.
    """

    def __init__(self, base_params: Any) -> None:
        self.base_params = base_params
        self._pairs: dict[int, tuple[Scenario, Any]] = {}
        self.fit_labels: tuple[str, ...] = ()

    def fit(self, construction: Sequence[Scenario]) -> Any:
        from fieldshift.indicators import ConstructionClimatology

        years: dict[int, np.ndarray] = {}
        for sc in construction:
            for i, y in enumerate(sc.years):
                years[y] = sc.precip[i * DAYS_PER_YEAR : (i + 1) * DAYS_PER_YEAR]
        ys = sorted(years)
        self.fit_labels = tuple(sc.label for sc in construction)
        ref = ConstructionClimatology(np.stack([years[y] for y in ys]), tuple(ys))
        return self.base_params.with_reference(ref)

    def fingerprint(self, params: Any) -> str:
        return params.reference.sha256()

    def components(self, plan: Any, scenario: Scenario, params: Any) -> tuple[float, float, float]:
        from fieldshift.indicators import ClimatePair, loss_components

        # VSDD M-1: o cache guarda o PRÓPRIO cenário junto do par. Enquanto está no cache o objeto
        # continua vivo, então seu id() não pode ser reaproveitado; a identidade é conferida.
        entry = self._pairs.get(id(scenario))
        if entry is None or entry[0] is not scenario:
            pair = ClimatePair(scenario.precip, scenario.tmax, scenario.tmin, scenario.label)
            entry = (scenario, pair)
            self._pairs[id(scenario)] = entry
        return loss_components(plan, entry[1], params)


def sbar_sha256_like_optimizer(climate: pd.DataFrame, protocol: EvalProtocol | None = None) -> str:
    """Hash de s̄ no mesmo formato do RunResult (concat P, TX, TN em float64), para auditoria."""
    split = split_scenarios(climate, protocol or EvalProtocol())
    sb = mean_climate(split.construction_years)
    return hashlib.sha256(np.concatenate([sb.precip, sb.tmax, sb.tmin]).tobytes()).hexdigest()
