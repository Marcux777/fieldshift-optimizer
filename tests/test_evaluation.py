# AI-assisted (OpenBot agent)
"""Testes do avaliador (protocolo v1.1): T3, T4, T5 (parte do avaliador), T6, T7."""

from __future__ import annotations

from dataclasses import dataclass

import numpy as np
import pandas as pd
import pytest

from fieldshift import evaluation as ev
from fieldshift.evaluation import DAYS_PER_YEAR, Scenario

# Fixtures SINTÉTICAS só para testes (nunca apresentadas como dado real).


def synthetic_climate(seed: int = 7) -> pd.DataFrame:
    dates = pd.date_range("1981-01-01", "2025-12-31", freq="D")
    rng = np.random.default_rng(seed)
    doy = dates.day_of_year.to_numpy()
    wet = 0.5 + 0.45 * np.cos(2 * np.pi * (doy - 15) / 365)  # chuva no verão austral
    rain = rng.gamma(0.6, 12.0, len(dates)) * (rng.random(len(dates)) < wet)
    tmax = 30 + 3 * np.sin(2 * np.pi * doy / 365) + rng.normal(0, 1.5, len(dates))
    tmin = tmax - 11 + rng.normal(0, 1.0, len(dates))
    return pd.DataFrame(
        {"PRECTOTCORR": rain, "T2M": (tmax + tmin) / 2, "T2M_MAX": tmax, "T2M_MIN": tmin},
        index=pd.DatetimeIndex(dates, name="date"),
    )


@dataclass(frozen=True)
class ToyPlan:
    id: tuple


TOY_PLANS = [ToyPlan((a, b)) for a in range(3) for b in range(3)]


class ToyLossModel:
    """Plano (a, b): a = janela de 60 dias no ano 1, b = no ano 2. g1 depende da chuva nessa
    janela relativa à mediana da CONSTRUÇÃO; g2 e g3 só do plano."""

    def __init__(self) -> None:
        self.fit_calls: list[tuple[str, ...]] = []

    @staticmethod
    def _window(a: int) -> slice:
        return slice(30 + 60 * a, 90 + 60 * a)

    def fit(self, construction):
        self.fit_calls.append(tuple(s.label for s in construction))
        med = {}
        for a in range(3):
            vals = [float(np.sum(s.precip[self._window(a)])) for s in construction]
            vals += [float(np.sum(s.precip[DAYS_PER_YEAR:][self._window(a)])) for s in construction]
            med[str(a)] = float(np.median(vals))
        return {"median": med}

    def components(self, plan, scenario: Scenario, params):
        a, b = plan.id
        p1 = float(np.sum(scenario.precip[self._window(a)]))
        p2 = float(np.sum(scenario.precip[DAYS_PER_YEAR:][self._window(b)]))
        e1 = max(0.0, 1 - p1 / params["median"][str(a)]) if params["median"][str(a)] else 0.0
        e2 = max(0.0, 1 - p2 / params["median"][str(b)]) if params["median"][str(b)] else 0.0
        g1 = (e1 + e2) / 2
        g2 = 0.1 * (a + b) / 4
        g3 = 0.0 if a != b else 0.5
        return (g1, g2, g3)


@pytest.fixture(scope="module")
def climate():
    return synthetic_climate()


def _policies_by_reference(climate, model, plans=TOY_PLANS, w=ev.DEFAULT_WEIGHTS):
    """Escolhe os planos como um otimizador honesto faria (só construção)."""
    split = ev.split_scenarios(climate, ev.EvalProtocol())
    params = model.fit(split.construction)
    sbar = ev.mean_climate(split.construction_years)
    Lc = ev.losses(ev.component_matrix(plans, split.construction, model, params), w)
    Lb = ev.losses(ev.component_matrix(plans, [sbar], model, params), w)[:, 0]
    ids = [p.id for p in plans]
    sel = ev.reference_selection(Lc, Lb, ids)
    return {k: plans[v] for k, v in sel.items()}


def _evaluate(climate, plans=TOY_PLANS, policy_plans=None, **kw):
    model = ToyLossModel()
    pp = policy_plans or {**_policies_by_reference(climate, ToyLossModel(), plans), "P0": plans[0]}
    return ev.evaluate_plans(
        plans=plans,
        policy_plans=pp,
        climate=climate,
        loss_model=model,
        protocol=ev.EvalProtocol(n_boot=2000),
        **kw,
    )


# --- T7 grade -----------------------------------------------------------------


def test_weight_grid_has_15_unique_points_summing_to_one_with_default():
    g = ev.weight_grid()
    assert len(g) == 15
    assert len(set(g)) == 15
    assert all(abs(sum(w) - 1) < 1e-12 and min(w) >= 0 for w in g)
    assert (0.50, 0.25, 0.25) in g
    assert g[0] == (1.0, 0.0, 0.0) and g[4] == (0.50, 0.25, 0.25) and g[-1] == (0.0, 0.0, 1.0)


# --- Split e calendário -------------------------------------------------------


def test_split_counts_and_boundary(climate):
    s = ev.split_scenarios(climate, ev.EvalProtocol())
    assert len(s.construction) == 28 and len(s.test) == 14
    assert s.construction[0].label == "1981/82–1982/83"
    assert s.construction[-1].label == "2008/09–2009/10"
    assert s.test[0].label == "2010/11–2011/12" and s.test[-1].label == "2023/24–2024/25"
    assert max(pd.Timestamp(x.last_date) for x in s.construction) < pd.Timestamp("2010-09-01")
    assert all(len(x.precip) == 730 for x in s.construction + s.test)
    assert [y.start for y in s.construction_years] == list(range(1981, 2010))


def test_feb29_dropped_every_year_has_365_days(climate):
    years = ev.build_agri_years(climate, 1983, 1984)  # 1984 é bissexto
    assert all(len(y.precip) == 365 and y.valid for y in years.values())
    y = years[1983]
    # t = 0 em 1/set; 28/fev é t = 180 e 1/mar é t = 181 (29/fev descartado)
    assert y.precip[180] == climate.loc["1984-02-28", "PRECTOTCORR"]
    assert y.precip[181] == climate.loc["1984-03-01", "PRECTOTCORR"]
    assert y.dates == ("1983-09-01", "1984-08-31")


# --- T6 ausentes --------------------------------------------------------------


def test_fill_value_becomes_missing_and_excludes_pairs_for_all(climate):
    c = climate.copy()
    c.loc["1990-01-15", "T2M_MAX"] = -999.0  # ano agrícola 1989/90
    s = ev.split_scenarios(c, ev.EvalProtocol())
    labels = [x.label for x in s.construction]
    assert "1988/89–1989/90" not in labels and "1989/90–1990/91" not in labels
    assert len(s.construction) == 26
    assert {e["scenario"] for e in s.excluded} == {"1988/89–1989/90", "1989/90–1990/91"}
    assert all(not np.isnan(x.tmax).any() for x in s.construction)
    # nunca 0: o ano 1989/90 não entra em s̄
    assert 1989 not in [y.start for y in s.construction_years]


def test_missing_day_in_index_is_missing_not_zero(climate):
    c = climate.drop(pd.Timestamp("2015-03-03"))
    s = ev.split_scenarios(c, ev.EvalProtocol())
    assert len(s.test) == 12
    assert any("2014/15" in e["scenario"] for e in s.excluded)


def test_mean_climate_aborts_when_a_day_has_no_value(climate):
    years = ev.build_agri_years(climate, 1981, 1983)
    broken = []
    for y in years.values():
        p = y.precip.copy()
        p[10] = np.nan
        broken.append(ev.AgriYear(y.start, y.dates, p, y.tmax, y.tmin, y.n_missing))
    with pytest.raises(ev.EvaluationError, match="s̄"):
        ev.mean_climate(broken)


def test_insufficient_data_outcome_first_line(climate):
    c = climate.copy()
    c.loc["1985-01-01":"1995-01-01", "PRECTOTCORR"] = np.nan
    r = _evaluate(c, policy_plans={k: TOY_PLANS[0] for k in ("P0", "P1", "P1b", "P2")})
    assert r.outcome == ev.OUTCOME_INSUFFICIENT
    assert r.to_markdown().startswith("**DESFECHO: DADOS_INSUFICIENTES**")


# --- T3 caso manual 3 planos x 3 cenários --------------------------------------


IDS = [("A",), ("B",), ("C",)]


def test_manual_mmr_conv_exp_differ():
    # L à mão. mins por cenário = [0, 0, 0.4]
    L = np.array([[0.0, 0.0, 0.9], [0.4, 0.4, 0.4], [0.5, 0.5, 0.5]])
    # R: A=[0,0,.5] máx .5 ; B=[.4,.4,0] máx .4 ; C=[.5,.5,.1] máx .5
    np.testing.assert_allclose(ev.regret(L), [[0, 0, 0.5], [0.4, 0.4, 0], [0.5, 0.5, 0.1]])
    assert ev.select_mmr(L, IDS) == 1  # B
    assert ev.select_exp(L, IDS) == 0  # A: média 0.3
    assert ev.select_conv(np.array([0.1, 0.3, 0.3]), L, IDS) == 0  # A em s̄


def test_manual_tie_break_levels():
    # nível 2: regret máx empata (.2), média desempata -> A
    L = np.array([[0.0, 0.2, 0.0], [0.2, 0.0, 0.1], [0.2, 0.2, 0.2]])
    assert ev.select_mmr(L, IDS) == 0
    # nível 3: A e B idênticos -> menor ID; ids fora de ordem
    L3 = np.array([[0.1, 0.3, 0.2], [0.1, 0.3, 0.2], [0.9, 0.9, 0.9]])
    ids = [("Z",), ("B",), ("C",)]
    assert ev.select_mmr(L3, ids) == 1
    assert ev.select_exp(L3, ids) == 1
    assert ev.select_conv(np.array([0.5, 0.5, 0.6]), L3, ids) == 1
    # CONV: L(s̄) empata em 9 casas, média desempata
    Lc = np.array([[0.2, 0.2, 0.2], [0.1, 0.1, 0.1], [0.3, 0.3, 0.3]])
    assert ev.select_conv(np.array([0.5 + 1e-11, 0.5, 0.4]), Lc, IDS) == 2
    assert ev.select_conv(np.array([0.5 + 1e-11, 0.5, 0.6]), Lc, IDS) == 1


def test_selection_invariant_to_enumeration_and_scenario_order():
    rng = np.random.default_rng(0)
    L = np.round(rng.random((7, 5)), 2)
    L[3] = L[5]  # empate exato
    ids = [(i,) for i in range(7)]
    Lb = np.round(rng.random(7), 2)
    base = {k: ids[v] for k, v in ev.reference_selection(L, Lb, ids).items()}
    for _ in range(20):
        pi, si = rng.permutation(7), rng.permutation(5)
        idp = [ids[i] for i in pi]
        sel = ev.reference_selection(L[pi][:, si], Lb[pi], idp)
        assert {k: idp[v] for k, v in sel.items()} == base


# --- T4 regret ------------------------------------------------------------------


def test_regret_nonnegative_and_zero_at_oracle():
    rng = np.random.default_rng(1)
    L = rng.random((20, 9))
    R = ev.regret(L)
    assert (R >= 0).all()
    np.testing.assert_array_equal(R[L.argmin(axis=0), np.arange(9)], 0.0)


# --- Estatística ----------------------------------------------------------------


def test_wilcoxon_exact_minimum_p_for_seven_pairs():
    r = ev.wilcoxon_exact(np.arange(1, 8, dtype=float))
    assert r["p_two_sided"] == pytest.approx(2 / 128)
    assert ev.wilcoxon_exact(np.zeros(7))["p_two_sided"] == 1.0
    sym = ev.wilcoxon_exact(np.array([1.0, -1.0, 2.0, -2.0]))
    assert sym["p_two_sided"] == 1.0


def test_block_bootstrap_deterministic_and_sign():
    d = np.array([-0.1] * 14)
    assert ev.block_bootstrap_ci(d, 2, 1000, 20261114) == (pytest.approx(-0.1), pytest.approx(-0.1))
    d2 = np.linspace(-0.05, 0.05, 14)
    a = ev.block_bootstrap_ci(d2, 2, 1000, 20261114)
    assert a == ev.block_bootstrap_ci(d2, 2, 1000, 20261114)
    assert a[0] < 0 < a[1]


def test_classification_rules():
    p = ev.EvalProtocol()
    c = ev.classify
    assert (
        c(identical=True, ci=(-1, -0.5), m1_a=0.1, m1_b=0.5, m0_ok=True, protocol=p)[0]
        == "INCONCLUSIVO"
    )
    assert (
        c(identical=False, ci=(-0.2, -0.05), m1_a=0.3, m1_b=0.5, m0_ok=True, protocol=p)[0]
        == "SUSTENTADO"
    )
    # ganho relativo < 10%
    assert (
        c(identical=False, ci=(-0.2, -0.05), m1_a=0.48, m1_b=0.5, m0_ok=True, protocol=p)[0]
        == "INCONCLUSIVO"
    )
    assert (
        c(identical=False, ci=(0.01, 0.2), m1_a=0.3, m1_b=0.5, m0_ok=True, protocol=p)[0]
        == "NEGATIVO"
    )
    assert (
        c(identical=False, ci=(-0.1, 0.1), m1_a=0.5, m1_b=0.5, m0_ok=True, protocol=p)[0]
        == "NEGATIVO"
    )
    assert (
        c(identical=False, ci=(-0.1, 0.1), m1_a=0.4, m1_b=0.5, m0_ok=True, protocol=p)[0]
        == "INCONCLUSIVO"
    )


# --- Avaliação ponta a ponta com fixtures ---------------------------------------


def test_evaluate_end_to_end_same_format_and_parity(climate):
    r = _evaluate(climate)
    assert r.outcome in {"SUSTENTADO", "NEGATIVO", "INCONCLUSIVO"}
    md = r.to_markdown()
    assert md.startswith(f"**DESFECHO: {r.outcome}**")
    assert r.M0_feasibility == 1.0
    assert all(r.audit["optimizer_matches_reference"].values())
    # paridade: todas as políticas medidas nos mesmos 14 cenários de teste
    assert {len(p.per_scenario_regret) for p in r.policies.values()} == {14}
    assert len(r.sensitivity) == 15 and sum(row["is_default"] for row in r.sensitivity) == 1
    assert [c.b for c in r.comparisons] == ["P1b", "P1"]


def test_infeasible_outcome_same_format(climate):
    r = ev.evaluate_plans(plans=[], policy_plans={}, climate=climate, loss_model=ToyLossModel())
    assert r.outcome == "INVIAVEL"
    assert r.to_markdown().startswith("**DESFECHO: INVIAVEL**")


def test_gate_m0_rejects_policy_plan_outside_X(climate):
    pp = {**_policies_by_reference(climate, ToyLossModel()), "P0": TOY_PLANS[0]}
    pp["P2"] = ToyPlan((9, 9))
    with pytest.raises(ev.EvaluationError, match="M0"):
        _evaluate(climate, policy_plans=pp)


def test_p0_outside_X_is_reported_and_excluded(climate):
    pp = {**_policies_by_reference(climate, ToyLossModel()), "P0": ToyPlan((7, 7))}
    r = _evaluate(climate, policy_plans=pp)
    assert r.p0_status == "inviavel_sob_ZARC_20"
    assert r.policies["P0"].feasible is False and r.policies["P0"].M1_max_regret is None
    assert "inviável sob ZARC" in r.to_markdown()


def test_evaluator_ignores_optimizer_scores(climate):
    pp = {**_policies_by_reference(climate, ToyLossModel()), "P0": TOY_PLANS[0]}
    honest = {"policies": {k: {"plan_id": list(v.id)} for k, v in pp.items()}}
    lying = {
        "policies": {
            k: {"plan_id": list(v.id), "loss": 0.0, "regret": [0.0] * 14} for k, v in pp.items()
        },
        "scores": {"P2": -1},
    }
    kw = dict(plans=TOY_PLANS, climate=climate, protocol=ev.EvalProtocol(n_boot=500))
    a = ev.evaluate(honest, loss_model=ToyLossModel(), **kw)
    b = ev.evaluate(lying, loss_model=ToyLossModel(), **kw)
    assert a.result_hash() == b.result_hash()


def test_audit_flags_optimizer_choice_that_breaks_protocol(climate):
    pp = {**_policies_by_reference(climate, ToyLossModel()), "P0": TOY_PLANS[0]}
    wrong = next(p for p in TOY_PLANS if p.id != pp["P2"].id)
    pp["P2"] = wrong
    r = _evaluate(climate, policy_plans=pp)
    assert r.audit["optimizer_matches_reference"]["P2"] is False


# --- T7 determinismo --------------------------------------------------------------


def test_two_runs_same_result_hash(climate):
    assert _evaluate(climate).result_hash() == _evaluate(climate).result_hash()


# --- Regressões da revisão VSDD -----------------------------------------------------


def test_M2_missing_T2M_excludes_scenario_like_the_other_variables(climate):
    c = climate.copy()
    c.loc["1990-01-15", "T2M"] = -999.0  # só T2M ausente, no ano agrícola 1989/90
    s = ev.split_scenarios(c, ev.EvalProtocol())
    labels = [x.label for x in s.construction]
    assert "1988/89–1989/90" not in labels and "1989/90–1990/91" not in labels
    assert len(s.construction) == 26


def test_M1_pair_cache_never_returns_climate_of_another_scenario():
    from test_leakage import ROOT

    from fieldshift import catalog as cat
    from fieldshift import indicators

    config = ROOT / "config"
    catalog = cat.load_catalog(config)
    soil = cat.load_soil(config)
    params = indicators.load_indicator_params(config, catalog)
    plans, _ = cat.enumerate_plans(catalog, soil, 0.20)
    plan = plans[0]
    rng = np.random.default_rng(3)
    model = ev.IndicatorLossModel(params)
    fresh_ref = None
    for k in range(60):
        # mesmo rótulo e mesmo start (como o s̄ do leave-one-out), conteúdo diferente;
        # o objeto anterior é descartado, então id() pode ser reaproveitado.
        p = rng.gamma(0.6, 12.0, 730)
        sc = ev.Scenario(
            start=-1,
            label="clima_medio_construcao",
            precip=p,
            tmax=np.full(730, 30.0),
            tmin=np.full(730, 19.0),
            first_date="",
            last_date="",
            years=(k,),
            synthetic=True,
        )
        if fresh_ref is None:
            fresh_ref = model.fit([sc])
        got = model.components(plan, sc, fresh_ref)
        want = ev.IndicatorLossModel(params).components(plan, sc, fresh_ref)
        assert got == want, f"iteração {k}: cache devolveu o clima de outro cenário"
        del sc
