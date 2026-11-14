# AI-assisted (OpenBot agent)
"""Indicadores g1 (exposição climática), g2 (cobertura vegetal) e g3 (diversidade), todos em [0, 1].

Função pura: ``loss_components(plan, climate_pair, params) -> (g1, g2, g3)``. Só precipitação
(``PRECTOTCORR``) e temperatura (``T2M_MAX``, ``T2M_MIN``) do NASA POWER; sem radiação, ET0 ou
balanço hídrico. "Menor é melhor". Os parâmetros vêm de ``config/indicators.toml`` (hipóteses do
Methods Scientist, congeladas antes da avaliação). Nenhum mínimo/máximo é tirado de dados: a única
estatística vinda de dados é a mediana de e3 (``ConstructionClimatology``), construída SÓ com os
anos de construção.

Definições (dia-índice t: 0 = 1/set de y; 365 dias/ano; ciclo vivo = [e, h) com e = s + emergência,
h = e + ciclo; fase crítica = [e + p1 + p2, e + p1 + p2 + p3), n_crit = p3):

* e1 = 1 se sum(P[s, s+10)) < 20 mm;
  e2 = maior sequência de dias com P < 1 mm na fase crítica / n_crit;
* e3 = max(0, 1 - Pcrit/Pref), Pref = mediana (construção) de Pcrit na mesma janela do ano agrícola;
* e4 = média dos componentes aplicáveis: calor (dias TX > limiar na fase crítica / n_crit) e geada
  (dias TN <= limiar [ou <] em [e, h) / ciclo);  e5 = 1 se >= 6 dias com P >= 1 mm em [h-10, h);
* eps = soma ponderada (pesos iguais) de e1..e5;  g1 = média de eps nas ocorrências de grão;
* g2 = 0,5 (1 - C1) + 0,5 C2/T (cobertura viva de calendário; falha de estabelecimento = 0 dias);
* g3 = 0,5 (4 - D1)/3 + 0,5 D2/3 (espécies distintas e repetições; só do plano).
"""

from __future__ import annotations

import hashlib
import math
import tomllib
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import numpy as np

from .catalog import DAYS_PER_YEAR, HORIZON_DAYS, Catalog, Crop, Plan

_TOL = 1e-12


class IndicatorError(ValueError):
    """Parâmetro ou entrada inválidos (por exemplo, dado ausente num cenário)."""


# --------------------------------------------------------------------------- clima
class ConstructionClimatology:
    """Precipitação dos anos agrícolas VÁLIDOS da construção (n, 365), só para a mediana de e3."""

    def __init__(self, precip_years: np.ndarray, labels: tuple[int, ...] = ()):
        arr = np.array(precip_years, dtype=np.float64, copy=True)
        if arr.ndim != 2 or arr.shape[1] != DAYS_PER_YEAR or arr.shape[0] < 1:
            raise IndicatorError(f"precip_years deve ter forma (n>=1, {DAYS_PER_YEAR})")
        if not np.isfinite(arr).all():
            raise IndicatorError("climatologia de construção com dado ausente")
        arr.setflags(write=False)
        self._p = arr
        self.labels = tuple(labels)
        self._memo: dict[tuple[int, int], float] = {}

    @property
    def n_years(self) -> int:
        return int(self._p.shape[0])

    def median_window_sum(self, lo: int, hi: int) -> float:
        """Mediana (numpy) da soma de P em [lo, hi) do ano agrícola, entre os anos de construção."""
        key = (lo, hi)
        if key not in self._memo:
            if not 0 <= lo < hi <= DAYS_PER_YEAR:
                raise IndicatorError(f"janela fora do ano agrícola: [{lo}, {hi})")
            self._memo[key] = float(np.median(self._p[:, lo:hi].sum(axis=1)))
        return self._memo[key]

    def sha256(self) -> str:
        return hashlib.sha256(self._p.tobytes()).hexdigest()


class ClimatePair:
    """Clima diário de um par de anos agrícolas (730 dias, sem 29/fev), sem nenhum dado ausente."""

    __slots__ = ("P", "TN", "TX", "_cache", "label")

    def __init__(self, P: np.ndarray, TX: np.ndarray, TN: np.ndarray, label: str = ""):
        arrs = []
        for name, a in (("P", P), ("TX", TX), ("TN", TN)):
            x = np.array(a, dtype=np.float64, copy=True)
            if x.shape != (HORIZON_DAYS,):
                raise IndicatorError(f"{name} deve ter {HORIZON_DAYS} dias, veio {x.shape}")
            if not np.isfinite(x).all():
                raise IndicatorError(f"{name} com dado ausente: cenário inválido não chega aqui")
            x.setflags(write=False)
            arrs.append(x)
        self.P, self.TX, self.TN = arrs
        self.label = label
        self._cache: dict[tuple[int, int], OccurrenceResult] = {}


# --------------------------------------------------------------------------- parâmetros
@dataclass(frozen=True)
class Thermal:
    heat_c: float | None = None
    frost_c: float | None = None
    frost_strict: bool = False


@dataclass(frozen=True)
class IndicatorParams:
    crops: tuple[Crop, ...]
    emergence_days: int
    brachiaria_desiccation_days: int
    thermal: Mapping[str, Thermal]
    e1_window_days: int
    e1_min_mm: float
    e2_dry_day_mm: float
    e5_window_days: int
    e5_rain_day_mm: float
    e5_rain_days_min: int
    component_weights: tuple[float, ...]
    horizon_days: int
    species_universe: tuple[str, ...]
    max_species: int
    max_repetitions: int
    default_weights: tuple[float, float, float]
    rounding_decimals: int
    sha256: str = ""
    reference: ConstructionClimatology | None = field(default=None, compare=False)

    def with_reference(self, ref: ConstructionClimatology) -> IndicatorParams:
        return IndicatorParams(**{**self.__dict__, "reference": ref})


def load_indicator_params(config_dir: Path, catalog: Catalog) -> IndicatorParams:
    path = Path(config_dir) / "indicators.toml"
    raw = path.read_bytes()
    cfg: dict[str, Any] = tomllib.loads(raw.decode("utf-8"))
    ex = cfg["exposure"]
    thermal: dict[str, Thermal] = {}
    for key, t in ex["e4"].items():
        if isinstance(t, dict):
            thermal[key] = Thermal(
                heat_c=t.get("heat_c"),
                frost_c=t.get("frost_c"),
                frost_strict=bool(t.get("frost_strict", False)),
            )
    cw = tuple(float(x) for x in ex["component_weights"])
    if len(cw) != 5 or abs(sum(cw) - 1.0) > _TOL or min(cw) < 0:
        raise IndicatorError("component_weights deve ter 5 pesos >= 0 somando 1")
    dw = cfg["loss"]["default_weights"]
    weights = (float(dw["exposure"]), float(dw["cover"]), float(dw["diversity"]))
    if abs(sum(weights) - 1.0) > _TOL:
        raise IndicatorError("default_weights deve somar 1")
    div = cfg["diversity"]
    if int(cfg["cover"]["T"]) != HORIZON_DAYS:
        raise IndicatorError("cover.T deve ser 730")
    return IndicatorParams(
        crops=catalog.crops,
        emergence_days=catalog.emergence_days,
        brachiaria_desiccation_days=catalog.brachiaria_desiccation_days,
        thermal=thermal,
        e1_window_days=int(ex["e1"]["window_days"]),
        e1_min_mm=float(ex["e1"]["establishment_min_mm"]),
        e2_dry_day_mm=float(ex["e2"]["dry_day_mm"]),
        e5_window_days=int(ex["e5"]["window_days"]),
        e5_rain_day_mm=float(ex["e5"]["rain_day_mm"]),
        e5_rain_days_min=int(ex["e5"]["rain_days_min"]),
        component_weights=cw,
        horizon_days=int(cfg["cover"]["T"]),
        species_universe=tuple(div["species_universe"]),
        max_species=int(div["max_species"]),
        max_repetitions=int(div["max_repetitions"]),
        default_weights=weights,
        rounding_decimals=int(cfg["loss"]["rounding_decimals"]),
        sha256=hashlib.sha256(raw).hexdigest(),
    )


# --------------------------------------------------------------------------- ocorrência
@dataclass(frozen=True)
class OccurrenceResult:
    e1: float
    e2: float
    e3: float
    e4: float
    e5: float
    eps: float

    def as_tuple(self) -> tuple[float, float, float, float, float]:
        return (self.e1, self.e2, self.e3, self.e4, self.e5)


def longest_true_run(mask: np.ndarray) -> int:
    """Maior sequência de True consecutivos."""
    if mask.size == 0 or not mask.any():
        return 0
    padded = np.concatenate(([0], mask.astype(np.int8), [0]))
    d = np.diff(padded)
    starts = np.flatnonzero(d == 1)
    ends = np.flatnonzero(d == -1)
    return int((ends - starts).max())


def crop_phase_bounds(
    crop: Crop, emergence_days: int, sow_t: int
) -> tuple[int, int, int, int, int]:
    """(emergência e, colheita h, início da fase crítica, fim da fase crítica, n_crit)."""
    p1, p2, p3, _p4 = crop.phase_days
    e = sow_t + emergence_days
    c0 = e + p1 + p2
    return e, e + crop.cycle_days, c0, c0 + p3, p3


def occurrence_indicators(
    pair: ClimatePair, params: IndicatorParams, crop_idx: int, sow_t: int
) -> OccurrenceResult:
    """e1..e5 e eps de uma ocorrência de cultura de grão (memoizado no par; função pura)."""
    key = (crop_idx, sow_t)
    hit = pair._cache.get(key)
    if hit is not None:
        return hit
    crop = params.crops[crop_idx]
    if crop.is_fallow or not crop.grain:
        raise IndicatorError(f"{crop.key} não é ocorrência de grão")
    if params.reference is None:
        raise IndicatorError("IndicatorParams sem climatologia de construção (e3 exige a mediana)")
    e, h, c0, c1, n_crit = crop_phase_bounds(crop, params.emergence_days, sow_t)
    if sow_t < 0 or h > HORIZON_DAYS:
        raise IndicatorError(f"ocupação fora do horizonte: s={sow_t}, h={h}")
    P, TX, TN = pair.P, pair.TX, pair.TN
    e1 = 1.0 if float(P[sow_t : sow_t + params.e1_window_days].sum()) < params.e1_min_mm else 0.0
    e2 = longest_true_run(P[c0:c1] < params.e2_dry_day_mm) / n_crit
    pcrit = float(P[c0:c1].sum())
    year0 = DAYS_PER_YEAR * (sow_t // DAYS_PER_YEAR)
    pref = params.reference.median_window_sum(c0 - year0, c1 - year0)
    e3 = 0.0 if pref == 0.0 else max(0.0, 1.0 - pcrit / pref)
    th = params.thermal.get(crop.key, Thermal())
    comps: list[float] = []
    if th.heat_c is not None:
        comps.append(float((TX[c0:c1] > th.heat_c).sum()) / n_crit)
    if th.frost_c is not None:
        seg = TN[e:h]
        frost = (seg < th.frost_c) if th.frost_strict else (seg <= th.frost_c)
        comps.append(float(frost.sum()) / crop.cycle_days)
    e4 = math.fsum(comps) / len(comps) if comps else 0.0
    rainy = int((P[h - params.e5_window_days : h] >= params.e5_rain_day_mm).sum())
    e5 = 1.0 if rainy >= params.e5_rain_days_min else 0.0
    w = params.component_weights
    eps = math.fsum((w[0] * e1, w[1] * e2, w[2] * e3, w[3] * e4, w[4] * e5))
    if not -_TOL <= eps <= 1.0 + _TOL:
        raise IndicatorError(f"eps fora de [0,1]: {eps}")
    res = OccurrenceResult(e1, e2, e3, e4, e5, min(1.0, max(0.0, eps)))
    pair._cache[key] = res
    return res


# --------------------------------------------------------------------------- g1, g2, g3
def exposure_g1(plan: Plan, pair: ClimatePair, params: IndicatorParams) -> float:
    vals = [
        occurrence_indicators(pair, params, o.crop, o.sow_t).eps  # type: ignore[arg-type]
        for o in plan.occupations
        if not o.is_fallow and params.crops[o.crop].grain
    ]
    if not vals:
        raise IndicatorError("plano sem cultura de grão")
    return math.fsum(vals) / len(vals)


def cover_mask(plan: Plan, pair: ClimatePair, params: IndicatorParams) -> np.ndarray:
    """Dias (0..T-1) com cobertura viva: [e, h) de cada ocorrência sem falha de estabelecimento,
    mais a braquiária do consórcio (de h até 15 d antes de J3; em J4 até o fim do horizonte)."""
    T = params.horizon_days
    cov = np.zeros(T, dtype=bool)
    s3 = plan.occupations[2].sow_t
    for o in plan.occupations:
        if o.is_fallow:
            continue
        crop = params.crops[o.crop]
        res = occurrence_indicators(pair, params, o.crop, o.sow_t)  # type: ignore[arg-type]
        if res.e1 == 1.0:
            continue
        e, h = o.emerge_t, o.harvest_t
        cov[e:h] = True  # type: ignore[misc]
        if crop.brachiaria_cover_after_harvest:
            end = T if o.window == "J4" else min(T, s3 - params.brachiaria_desiccation_days)  # type: ignore[operator]
            if end > h:  # type: ignore[operator]
                cov[h:end] = True  # type: ignore[misc]
    return cov


def cover_g2(plan: Plan, pair: ClimatePair, params: IndicatorParams) -> float:
    T = params.horizon_days
    cov = cover_mask(plan, pair, params)
    c1 = float(cov.sum()) / T
    c2 = longest_true_run(~cov)
    return 0.5 * (1.0 - c1) + 0.5 * c2 / T


def diversity_terms(plan: Plan, params: IndicatorParams) -> tuple[int, int]:
    """(D1 espécies distintas, D2 repetições r1+r2+r3)."""
    crops = [params.crops[o.crop] for o in plan.occupations]
    species: set[str] = set()
    for c in crops:
        species.update(c.species)
    unknown = species - set(params.species_universe)
    if unknown:
        raise IndicatorError(f"espécies fora do universo: {sorted(unknown)}")
    o = plan.occupations
    r1 = int(o[0].crop == o[2].crop)
    r2 = int(o[1].crop == o[3].crop and not o[1].is_fallow)
    r3 = int(
        any(
            "milho" in crops[a].species and "milho" in crops[b].species for a, b in ((0, 1), (2, 3))
        )
    )
    return len(species), r1 + r2 + r3


def diversity_g3(plan: Plan, params: IndicatorParams) -> float:
    d1, d2 = diversity_terms(plan, params)
    if not 1 <= d1 <= params.max_species or d2 > params.max_repetitions:
        raise IndicatorError(f"D1={d1} ou D2={d2} fora do intervalo teórico")
    return (
        0.5 * (params.max_species - d1) / (params.max_species - 1)
        + 0.5 * d2 / params.max_repetitions
    )


def loss_components(
    plan: Plan, climate_pair: ClimatePair, params: IndicatorParams
) -> tuple[float, float, float]:
    """(g1, g2, g3) do plano no par de anos agrícolas. Função pura; todos em [0, 1]."""
    g = (
        exposure_g1(plan, climate_pair, params),
        cover_g2(plan, climate_pair, params),
        diversity_g3(plan, params),
    )
    for v in g:
        if not -_TOL <= v <= 1.0 + _TOL:
            raise IndicatorError(f"g fora de [0,1]: {g}")
    return (min(1.0, max(0.0, g[0])), min(1.0, max(0.0, g[1])), min(1.0, max(0.0, g[2])))
