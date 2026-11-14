# AI-assisted (OpenBot agent)
"""Testes dos indicadores g1, g2, g3 (valores calculados à mão e avaliador ingênuo independente)."""

from __future__ import annotations

import math
from pathlib import Path

import numpy as np
import pytest

from fieldshift import catalog as C
from fieldshift import indicators as I  # noqa: E741

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "config"
T = 730


@pytest.fixture(scope="module")
def cat() -> C.Catalog:
    return C.load_catalog(CONFIG)


@pytest.fixture(scope="module")
def soil() -> C.Soil:
    return C.load_soil(CONFIG)


@pytest.fixture(scope="module")
def plans(cat, soil):
    return C.enumerate_plans(cat, soil)[0]


def make_params(cat, ref_value: float = 5.0, n_years: int = 5) -> I.IndicatorParams:
    base = I.load_indicator_params(CONFIG, cat)
    ref = I.ConstructionClimatology(np.full((n_years, 365), ref_value))
    return base.with_reference(ref)


def pair(P=5.0, TX=30.0, TN=15.0) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    return (
        np.full(T, P, dtype=float),
        np.full(T, TX, dtype=float),
        np.full(T, TN, dtype=float),
    )


def find(cat, plans, crops, decs=None):
    for p in plans:
        keys = tuple(cat.crops[o.crop].key for o in p.occupations)
        if keys == crops and (decs is None or (p.occupations[0].dec, p.occupations[2].dec) == decs):
            return p
    raise AssertionError(crops)


# ------------------------------------------------------------------ ocorrência (soja, J1 em t=40)
def test_soja_occurrence_hand_computed(cat):
    prm = make_params(cat, ref_value=2.0)
    P, TX, TN = pair(P=2.0)
    soy = cat.index_of("soja")
    # soja semeada em t=40: e=47, h=147; fase crítica [97, 127) (n=30); janela e1 = [40, 50)
    P[100:112] = 0.0  # veranico de 12 dias na fase crítica
    TX[[100, 101, 102]] = 41.0  # 3 dias > 40 °C na fase crítica
    TN[[50, 51, 52, 53, 54]] = 3.0  # <= 3 °C (conta, limiar não estrito)
    TN[[60, 61]] = 3.01  # não conta
    P[137:147] = 0.0
    P[[137, 138, 139, 140, 141, 142]] = 1.0  # 6 dias >= 1 mm no último decêndio -> e5 = 1
    r = I.occurrence_indicators(I.ClimatePair(P, TX, TN), prm, soy, 40)
    assert r.e1 == 0.0  # sum(P[40:50]) = 20 >= 20
    assert r.e2 == pytest.approx(12 / 30)
    assert r.e3 == pytest.approx(1 - 36 / 60)  # Pcrit = 2*(30-12) = 36; Pref = 2*30 = 60
    assert r.e4 == pytest.approx((3 / 30 + 5 / 100) / 2)
    assert r.e5 == 1.0
    assert r.eps == pytest.approx((0 + 12 / 30 + 0.4 + (0.1 + 0.05) / 2 + 1) / 5)


def test_e1_threshold_and_e5_threshold(cat):
    prm = make_params(cat, ref_value=2.0)
    soy = cat.index_of("soja")
    P, TX, TN = pair(P=2.0)
    P[40:50] = 1.9  # 19 mm < 20 -> falha de estabelecimento
    P[137:147] = 0.0
    P[[137, 138, 139, 140, 141]] = 1.0  # só 5 dias chuvosos -> e5 = 0
    r = I.occurrence_indicators(I.ClimatePair(P, TX, TN), prm, soy, 40)
    assert r.e1 == 1.0 and r.e5 == 0.0


def test_sorgo_frost_is_strict_and_heat(cat):
    prm = make_params(cat, ref_value=5.0)
    sg = cat.index_of("sorgo_2a")
    # sorgo semeado em 147: e=154, h=264, fase crítica [154+16+39, +33) = [209, 242)
    P, TX, TN = pair(P=5.0)
    TN[[160, 161, 162]] = 2.0  # == 2.0: não conta (estrito)
    TN[[170, 171]] = 1.99  # conta
    TX[[210, 211, 212, 213]] = 38.5  # > 38 na fase crítica
    r = I.occurrence_indicators(I.ClimatePair(P, TX, TN), prm, sg, 147)
    assert r.e4 == pytest.approx((4 / 33 + 2 / 110) / 2)


def test_milho_has_only_frost_component(cat):
    prm = make_params(cat, ref_value=5.0)
    mz = cat.index_of("milho_2a")
    P, TX, TN = pair(P=5.0, TX=45.0)  # calor não conta para milho (sem limiar verificado)
    TN[[160, 161]] = 2.0  # <= 2 conta (milho não estrito); ciclo [154, 264) = 110 d
    r = I.occurrence_indicators(I.ClimatePair(P, TX, TN), prm, mz, 147)
    assert r.e4 == pytest.approx(2 / 110)


def test_e3_zero_when_reference_zero_and_never_negative(cat):
    soy = cat.index_of("soja")
    P, TX, TN = pair(P=50.0)
    prm0 = make_params(cat, ref_value=0.0)
    assert I.occurrence_indicators(I.ClimatePair(P, TX, TN), prm0, soy, 40).e3 == 0.0
    prm = make_params(cat, ref_value=1.0)
    assert (
        I.occurrence_indicators(I.ClimatePair(P, TX, TN), prm, soy, 40).e3 == 0.0
    )  # chuva acima da mediana


def test_e3_uses_same_window_in_second_agri_year(cat):
    soy = cat.index_of("soja")
    ref = np.full((4, 365), 1.0)
    ref[:, 97:127] = 2.0  # janela crítica de uma soja semeada em t=40 do ano agrícola
    prm = I.load_indicator_params(CONFIG, cat).with_reference(I.ConstructionClimatology(ref))
    P, TX, TN = pair(P=1.0)
    P[365 + 97 : 365 + 127] = 1.0  # mesma janela no ano 2 (soja semeada em 405 = 365 + 40)
    r = I.occurrence_indicators(I.ClimatePair(P, TX, TN), prm, soy, 405)
    assert r.e3 == pytest.approx(1 - 30 / 60)


def test_reference_median_is_numpy_median():
    rng = np.random.default_rng(1)
    ref = rng.uniform(0, 10, size=(6, 365))
    cl = I.ConstructionClimatology(ref)
    assert cl.median_window_sum(100, 130) == pytest.approx(
        float(np.median(ref[:, 100:130].sum(axis=1)))
    )
    ref2 = ref.copy()
    ref2[:, :5] = 999  # alterar dias fora da janela não muda a mediana
    assert I.ConstructionClimatology(ref2).median_window_sum(100, 130) == cl.median_window_sum(
        100, 130
    )


def test_missing_data_never_reaches_indicators():
    P, TX, TN = pair()
    P[10] = np.nan
    with pytest.raises(I.IndicatorError, match="ausente"):
        I.ClimatePair(P, TX, TN)
    with pytest.raises(I.IndicatorError):
        I.ConstructionClimatology(np.full((3, 365), np.nan))
    with pytest.raises(I.IndicatorError):
        I.ClimatePair(*[np.zeros(729)] * 3)


# ------------------------------------------------------------------ g1, g2, g3 de planos conhecidos
def test_p0_values_wet_climate(cat, plans):
    """Clima uniforme 5 mm/d: e1=e2=e3=e4=0 e e5=1 (10 dias chuvosos) => eps=0,2 e g1=0,2."""
    prm = make_params(cat, ref_value=5.0)
    p0 = find(cat, plans, ("soja", "milho_2a", "soja", "milho_2a"), (29, 29))
    g1, g2, g3 = I.loss_components(p0, I.ClimatePair(*pair()), prm)
    assert g1 == pytest.approx(0.2)
    # cobertura: [47,147) + [154,264) + [412,512) + [519,629) = 420 d; maior descoberto = 148
    assert g2 == pytest.approx(0.5 * (310 / 730) + 0.5 * (148 / 730))
    assert g3 == pytest.approx(2 / 3)  # D1 = 2 (soja, milho); D2 = 2 (soja-soja, milho-milho)


def test_dry_climate_all_failed(cat, plans):
    prm = make_params(cat, ref_value=5.0)
    p0 = find(cat, plans, ("soja", "milho_2a", "soja", "milho_2a"), (29, 29))
    g1, g2, _ = I.loss_components(p0, I.ClimatePair(*pair(P=0.0)), prm)
    # e1=1, e2=1, e3=1, e4=0, e5=0 -> eps=0,6; sem cobertura nenhuma: g2 = 0,5 + 0,5
    assert g1 == pytest.approx(0.6)
    assert g2 == pytest.approx(1.0)


def test_brachiaria_cover_hand_computed(cat, plans):
    prm = make_params(cat, ref_value=5.0)
    p = find(cat, plans, ("soja", "milho_2a_braquiaria", "soja", "milho_2a_braquiaria"), (28, 28))
    _, g2, _ = I.loss_components(p, I.ClimatePair(*pair()), prm)
    # [37,137) 100 d; [144,380) 236 d (milho 115 d + braquiária até 15 d antes de J3 = 395-15);
    # [402,502) 100 d; [509,730) 221 d. Cobertos 657; descobertos 73; maior intervalo 37.
    assert g2 == pytest.approx(0.5 * (73 / 730) + 0.5 * (37 / 730))


def test_establishment_failure_removes_cover_including_brachiaria(cat, plans):
    prm = make_params(cat, ref_value=5.0)
    p = find(cat, plans, ("soja", "milho_2a_braquiaria", "soja", "milho_2a_braquiaria"), (28, 28))
    P, TX, TN = pair()
    P[137:147] = 0.0  # J2 semeia em 137: falha de estabelecimento (e1=1) só no J2 do ano 1
    _, g2, _ = I.loss_components(p, I.ClimatePair(P, TX, TN), prm)
    # coberto: 100 (J1) + 100 (J3) + 221 (J4 + braquiária) = 421; descoberto 309; maior 265
    assert g2 == pytest.approx(0.5 * (309 / 730) + 0.5 * (265 / 730))


def test_g3_extremes(cat, plans):
    prm = make_params(cat)
    diverse = find(cat, plans, ("soja", "milho_2a_braquiaria", "milho_1a", "sorgo_2a"))
    assert I.diversity_terms(diverse, prm) == (4, 0)
    assert I.diversity_g3(diverse, prm) == 0.0
    worst = find(cat, plans, ("milho_1a", "milho_2a", "milho_1a", "milho_2a"), (30, 30))
    assert I.diversity_terms(worst, prm) == (1, 3)  # milho-milho nos dois anos + r1 + r2
    assert I.diversity_g3(worst, prm) == pytest.approx(1.0)


def test_g1_is_mean_of_grain_occurrences_not_sum(cat, plans):
    """Plano com pousio em J2/J4 tem média sobre 2 culturas (não premia o pousio)."""
    prm = make_params(cat, ref_value=5.0)
    fall = next(p for p in plans if len(p.fallow_windows()) == 2)
    pr = I.ClimatePair(*pair())
    eps = [
        I.occurrence_indicators(pr, prm, o.crop, o.sow_t).eps
        for o in fall.occupations
        if not o.is_fallow
    ]
    assert len(eps) == 2
    assert I.exposure_g1(fall, pr, prm) == pytest.approx(sum(eps) / 2)
    # sob clima uniforme todas as ocorrências têm eps=0,2: g1 não depende do número de culturas
    assert I.exposure_g1(fall, pr, prm) == pytest.approx(0.2)


# ------------------------------------------------------------------ avaliador ingênuo independente
def naive_loss(cat, plan, P, TX, TN, ref_years, prm):
    """Reimplementação direta da especificação, com laços (independente de indicators.py)."""
    emerg = 7
    eps_by = {}
    covered = [False] * T
    eps_list = []
    for o in plan.occupations:
        if o.is_fallow:
            continue
        crop = cat.crops[o.crop]
        s = o.sow_t
        e = s + emerg
        h = e + crop.cycle_days
        p1, p2, p3, _ = crop.phase_days
        c0, c1 = e + p1 + p2, e + p1 + p2 + p3
        e1 = 1.0 if sum(P[s : s + 10]) < 20.0 else 0.0
        run = best = 0
        for t in range(c0, c1):
            run = run + 1 if P[t] < 1.0 else 0
            best = max(best, run)
        e2 = best / p3
        y0 = 365 if s >= 365 else 0
        sums = sorted(sum(row[c0 - y0 : c1 - y0]) for row in ref_years)
        n = len(sums)
        pref = sums[n // 2] if n % 2 else (sums[n // 2 - 1] + sums[n // 2]) / 2
        pcrit = sum(P[c0:c1])
        e3 = 0.0 if pref == 0 else max(0.0, 1 - pcrit / pref)
        th = prm.thermal.get(crop.key, I.Thermal())
        comps = []
        if th.heat_c is not None:
            comps.append(sum(1 for t in range(c0, c1) if TX[t] > th.heat_c) / p3)
        if th.frost_c is not None:
            f = [
                1
                for t in range(e, h)
                if (TN[t] < th.frost_c if th.frost_strict else TN[t] <= th.frost_c)
            ]
            comps.append(len(f) / crop.cycle_days)
        e4 = sum(comps) / len(comps) if comps else 0.0
        e5 = 1.0 if sum(1 for t in range(h - 10, h) if P[t] >= 1.0) >= 6 else 0.0
        eps = (e1 + e2 + e3 + e4 + e5) / 5
        eps_list.append(eps)
        eps_by[o.window] = (e1, e, h)
        if e1 == 0.0:
            for t in range(e, h):
                covered[t] = True
            if crop.brachiaria_cover_after_harvest:
                end = T if o.window == "J4" else min(T, plan.occupations[2].sow_t - 15)
                for t in range(h, end):
                    covered[t] = True
    g1 = sum(eps_list) / len(eps_list)
    c1v = sum(covered) / T
    run = best = 0
    for c in covered:
        run = 0 if c else run + 1
        best = max(best, run)
    g2 = 0.5 * (1 - c1v) + 0.5 * best / T
    species = set()
    for o in plan.occupations:
        species.update(cat.crops[o.crop].species)
    keys = [cat.crops[o.crop].key for o in plan.occupations]
    r1 = int(keys[0] == keys[2])
    r2 = int(keys[1] == keys[3] and keys[1] != "pousio")
    mil = lambda k: k in ("milho_1a", "milho_2a", "milho_2a_braquiaria")  # noqa: E731
    r3 = int((mil(keys[0]) and mil(keys[1])) or (mil(keys[2]) and mil(keys[3])))
    g3 = 0.5 * (4 - len(species)) / 3 + 0.5 * (r1 + r2 + r3) / 3
    return g1, g2, g3


@pytest.mark.parametrize("seed", [11, 12, 13])
def test_vectorized_equals_naive_on_random_climates(cat, plans, seed):
    rng = np.random.default_rng(seed)
    ref = rng.gamma(0.4, 6.0, size=(7, 365)) * (rng.random((7, 365)) < 0.45)
    prm = I.load_indicator_params(CONFIG, cat).with_reference(I.ConstructionClimatology(ref))
    P = rng.gamma(0.4, 6.0, size=T) * (rng.random(T) < 0.45)
    TX = rng.normal(31, 4, size=T)
    TN = rng.normal(16, 5, size=T)
    pr = I.ClimatePair(P, TX, TN)
    sample = plans[:: max(1, len(plans) // 60)]
    assert len(sample) >= 50
    for p in sample:
        got = I.loss_components(p, pr, prm)
        want = naive_loss(cat, p, P, TX, TN, ref, prm)
        assert got == pytest.approx(want, abs=1e-9), p.id


@pytest.mark.parametrize("kind", ["zeros", "flood", "hot", "cold", "random"])
def test_all_components_in_unit_interval(cat, plans, kind):
    prm = make_params(cat, ref_value=4.0)
    rng = np.random.default_rng(5)
    P, TX, TN = pair()
    if kind == "zeros":
        P[:] = 0
    elif kind == "flood":
        P[:] = 500
    elif kind == "hot":
        TX[:] = 50
    elif kind == "cold":
        TN[:] = -10
    else:
        P, TX, TN = rng.gamma(0.3, 9, T), rng.normal(30, 9, T), rng.normal(12, 9, T)
    pr = I.ClimatePair(P, TX, TN)
    for p in plans:
        g = I.loss_components(p, pr, prm)
        assert all(0.0 <= v <= 1.0 for v in g) and all(math.isfinite(v) for v in g)


def test_pure_function_same_input_same_output(cat, plans):
    prm = make_params(cat)
    rng = np.random.default_rng(9)
    P, TX, TN = rng.gamma(0.4, 6, T), rng.normal(31, 3, T), rng.normal(16, 3, T)
    p = plans[100]
    a = I.loss_components(p, I.ClimatePair(P, TX, TN), prm)
    b = I.loss_components(p, I.ClimatePair(P, TX, TN), prm)  # cache novo
    pr = I.ClimatePair(P, TX, TN)
    c = [I.loss_components(p, pr, prm) for _ in range(3)]
    assert a == b == c[0] == c[1] == c[2]


def test_reference_required_for_e3(cat, plans):
    prm = I.load_indicator_params(CONFIG, cat)  # sem climatologia
    with pytest.raises(I.IndicatorError, match="climatologia"):
        I.loss_components(plans[0], I.ClimatePair(*pair()), prm)


def test_params_loaded_from_config(cat):
    prm = I.load_indicator_params(CONFIG, cat)
    assert prm.default_weights == (0.5, 0.25, 0.25)
    assert prm.component_weights == (0.2,) * 5
    assert prm.thermal["sorgo_2a"].frost_strict is True
    assert prm.thermal["soja"].heat_c == 40.0 and prm.thermal["milho_1a"].heat_c is None
    assert len(prm.sha256) == 64
