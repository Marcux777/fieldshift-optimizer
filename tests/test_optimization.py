# AI-assisted (OpenBot agent)
"""Testes de otimização: grade, minimax regret, desempate, Pareto, cenários, ausentes, T2."""

from __future__ import annotations

import itertools
import json
import time
from dataclasses import replace
from pathlib import Path

import numpy as np
import pandas as pd
import pytest

from fieldshift import catalog as C
from fieldshift import data
from fieldshift import indicators as I  # noqa: E741
from fieldshift import optimization as O  # noqa: E741

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "config"


# ------------------------------------------------------------------ fixtures
@pytest.fixture(scope="module")
def cat() -> C.Catalog:
    return C.load_catalog(CONFIG)


@pytest.fixture(scope="module")
def soil() -> C.Soil:
    return C.load_soil(CONFIG)


@pytest.fixture(scope="module")
def params(cat) -> I.IndicatorParams:
    return I.load_indicator_params(CONFIG, cat)


def synth_climate(seed: int = 0) -> pd.DataFrame:
    """Clima SINTÉTICO só para testes (1981-01-01..2025-12-31); nunca apresentado como real."""
    idx = pd.date_range("1981-01-01", "2025-12-31", freq="D", name="date")
    rng = np.random.default_rng(seed)
    doy = idx.day_of_year.to_numpy()
    wet = 0.5 + 0.45 * np.cos(2 * np.pi * (doy - 15) / 365)  # chuva de out a mar
    P = rng.gamma(0.5, 12.0, size=len(idx)) * (rng.random(len(idx)) < wet)
    TX = 30 + 3 * np.sin(2 * np.pi * (doy - 80) / 365) + rng.normal(0, 2, len(idx))
    TN = 16 + 4 * np.sin(2 * np.pi * (doy - 80) / 365) + rng.normal(0, 2, len(idx))
    return pd.DataFrame(
        {"PRECTOTCORR": P, "T2M": (TX + TN) / 2, "T2M_MAX": TX, "T2M_MIN": TN}, index=idx
    )


@pytest.fixture(scope="module")
def climate() -> pd.DataFrame:
    return synth_climate()


@pytest.fixture(scope="module")
def synth_run(cat, soil, params, climate):
    return O.run(climate, cat, soil, params, snapshot_id="synthetic-test")


@pytest.fixture(scope="module")
def real(cat, soil, params):
    snap = data.latest_snapshot(ROOT / "data" / "snapshots")
    clim = data.load_climate(snap)
    t0 = time.perf_counter()
    res = O.run(clim, cat, soil, params, snapshot_id=snap.name)
    return res, clim, snap, time.perf_counter() - t0


# ------------------------------------------------------------------ grade de 15 pesos
EXPECTED_GRID = [
    (1.0, 0.0, 0.0), (0.75, 0.25, 0.0), (0.75, 0.0, 0.25), (0.5, 0.5, 0.0), (0.5, 0.25, 0.25),
    (0.5, 0.0, 0.5), (0.25, 0.75, 0.0), (0.25, 0.5, 0.25), (0.25, 0.25, 0.5), (0.25, 0.0, 0.75),
    (0.0, 1.0, 0.0), (0.0, 0.75, 0.25), (0.0, 0.5, 0.5), (0.0, 0.25, 0.75), (0.0, 0.0, 1.0),
]  # fmt: skip


def test_weight_grid_is_15_unique_triples_in_quarters():
    g = list(O.WEIGHT_GRID)
    assert g == EXPECTED_GRID
    assert len(set(g)) == 15
    assert all(sum(w) == 1.0 for w in g)
    assert all(x * 4 == int(x * 4) for w in g for x in w)
    assert (0.5, 0.25, 0.25) in g and O.DEFAULT_WEIGHTS == (0.5, 0.25, 0.25)
    # contagem independente: triplas (a,b,c) inteiras >= 0 com a+b+c = 4
    n = sum(1 for t in itertools.product(range(5), repeat=3) if sum(t) == 4)
    assert n == 15 == len(g)


# ------------------------------------------------------------------ minimax regret e desempate
IDS = [(0, 2), (0, 1), (0, 3)]


def test_toy_3x3_by_hand():
    L = np.array([[1.0, 3.0, 2.0], [2.0, 2.0, 2.0], [3.0, 1.0, 4.0]])  # 3 planos x 3 cenários
    R = O.regret_matrix(L)
    assert R.tolist() == [[0.0, 2.0, 0.0], [1.0, 1.0, 0.0], [2.0, 0.0, 2.0]]  # oráculo [1, 1, 2]
    assert O.select_mmr(L, IDS).index == 1  # max R = (2, 1, 2): plano B
    sel = O.select_exp(L, IDS)  # médias L = (2, 2, 8/3): empate A/B; desempata por max R (A=2, B=1)
    assert sel.index == 1 and sel.tied_after_level == (2, 1) and not sel.id_tiebreak
    Lbar = np.array([1.5, 1.5, 3.0])
    conv = O.select_conv(
        L, Lbar, IDS
    )  # empata em L(s_barra) e em média L: decide o ID (0,1) < (0,2)
    assert conv.index == 1 and conv.id_tiebreak and conv.tied_after_level == (2, 2)
    ids2 = [(0, 1), (0, 2), (0, 3)]
    assert O.select_conv(L, Lbar, ids2).index == 0


def test_mmr_second_criterion_mean_regret():
    # oráculo = 0 em todos os cenários; max R = 2 empata entre A, B e C; média R: C=1 vence
    L = np.array([[0.0, 2.0, 2.0], [2.0, 2.0, 0.0], [2.0, 0.0, 1.0]])
    assert O.regret_matrix(L).tolist() == L.tolist()
    sel = O.select_mmr(L, [(0, 1), (0, 2), (0, 3)])
    assert sel.index == 2 and sel.tied_after_level == (3, 1) and not sel.id_tiebreak


def test_numeric_tolerance_is_nine_decimals_then_id():
    # B é pior por 1e-12 (abaixo da tolerância de 9 casas): empata em tudo e o ID decide
    L = np.array([[0.1, 0.5], [0.1 + 1e-12, 0.5]])
    sel = O.select_mmr(L, [(0, 2), (0, 1)])
    assert sel.index == 1 and sel.id_tiebreak
    # acima da tolerância (1e-6) o critério numérico decide
    L2 = np.array([[0.1, 0.5], [0.1 + 1e-6, 0.5]])
    assert O.select_mmr(L2, [(0, 2), (0, 1)]).index == 0


def test_regret_nonnegative_and_oracle_zero():
    rng = np.random.default_rng(3)
    L = rng.random((40, 9))
    R = O.regret_matrix(L)
    assert (R >= 0).all()
    assert (R.min(axis=0) == 0).all()  # algum plano é oráculo em cada cenário
    assert np.allclose(R, L - L.min(axis=0))


@pytest.mark.parametrize("seed", [0, 1, 2, 3])
def test_selection_invariant_to_plan_and_scenario_order(seed):
    rng = np.random.default_rng(seed)
    n, s = 30, 8
    L = rng.integers(0, 4, size=(n, s)) / 4.0  # muitos empates exatos
    Lbar = rng.integers(0, 4, size=n) / 4.0
    ids = [(int(i // 6), int(i % 6)) for i in range(n)]
    base = (
        ids[O.select_mmr(L, ids).index],
        ids[O.select_exp(L, ids).index],
        ids[O.select_conv(L, Lbar, ids).index],
    )
    for _ in range(6):
        pp = rng.permutation(n)
        ss = rng.permutation(s)
        L2 = L[pp][:, ss]
        ids2 = [ids[i] for i in pp]
        got = (
            ids2[O.select_mmr(L2, ids2).index],
            ids2[O.select_exp(L2, ids2).index],
            ids2[O.select_conv(L2, Lbar[pp], ids2).index],
        )
        assert got == base


def test_loss_matrix_fixed_order_and_weights():
    comp = np.array([[[0.2, 0.4, 0.6]]])
    assert O.loss_matrix(comp, (0.5, 0.25, 0.25))[0, 0] == 0.5 * 0.2 + 0.25 * 0.4 + 0.25 * 0.6
    assert O.loss_matrix(comp, (0.0, 0.0, 1.0))[0, 0] == 0.6


# ------------------------------------------------------------------ Pareto
def _naive_front(F):
    n = len(F)
    nd = []
    for i in range(n):
        dom = any(
            all(F[j][k] <= F[i][k] for k in range(F.shape[1]))
            and any(F[j][k] < F[i][k] for k in range(F.shape[1]))
            for j in range(n)
        )
        if not dom:
            nd.append(i)
    return nd


@pytest.mark.parametrize("seed", range(5))
def test_pareto_matches_bruteforce_with_duplicates(seed):
    rng = np.random.default_rng(seed)
    F = rng.integers(0, 5, size=(60, 4)).astype(float) / 4
    ids = [(i,) for i in rng.permutation(60).tolist()]
    front, eq = O.pareto_front(F, ids)
    naive = _naive_front(F)
    assert {tuple(F[i]) for i in front} == {tuple(F[i]) for i in naive}  # mesmos vetores
    for r in front:
        same = [i for i in range(60) if tuple(F[i]) == tuple(F[r])]
        assert r == min(same, key=lambda i: ids[i])  # representante = menor ID
        assert set(eq[r]) == set(same) - {r}
        assert eq[r] == sorted(eq[r], key=lambda i: ids[i])
    for i in range(60):  # nenhum fora da fronteira é não dominado
        if tuple(F[i]) not in {tuple(F[r]) for r in front}:
            assert i not in naive


def test_pareto_strict_dominance_example():
    F = np.array([[1, 1], [1, 2], [2, 1], [1, 1], [0, 3], [3, 0]], dtype=float)
    ids = [(i,) for i in range(6)]
    front, eq = O.pareto_front(F, ids)
    assert front == [
        0,
        4,
        5,
    ]  # (1,2) e (2,1) dominados por (1,1); (1,1) duplicado -> representante 0
    assert eq[0] == [3]


# ------------------------------------------------------------------ cenários, split e ausentes
def test_scenarios_split_and_labels(climate):
    c = O.build_scenarios(climate, "construction")
    t = O.build_scenarios(climate, "test")
    assert (len(c.scenarios), len(c.valid), c.n_expected) == (28, 28, 28)
    assert (len(t.scenarios), len(t.valid), t.n_expected) == (14, 14, 14)
    assert c.scenarios[0].label == "1981/82+1982/83" and c.scenarios[-1].label == "2008/09+2009/10"
    assert t.scenarios[0].label == "2010/11+2011/12" and t.scenarios[-1].label == "2023/24+2024/25"
    assert max(s.y for s in c.scenarios) + 1 < O.TEST_FIRST_Y  # nenhum par cruza a fronteira
    assert min(s.y for s in t.scenarios) >= O.TEST_FIRST_Y
    assert all(s.pair.P.shape == (730,) for s in c.valid + t.valid)


def test_agri_year_alignment_and_feb29_dropped(climate):
    clim = climate.copy()
    clim.loc["1984-02-29", "PRECTOTCORR"] = 12345.0
    c = O.build_scenarios(clim, "construction")
    s0 = c.scenarios[0].pair  # par 1981/82 + 1982/83
    P = climate["PRECTOTCORR"]
    assert (
        s0.P[0] == P["1981-09-01"] and s0.P[122] == P["1982-01-01"] and s0.P[364] == P["1982-08-31"]
    )
    assert s0.P[365] == P["1982-09-01"] and s0.P[729] == P["1983-08-31"]
    assert all(12345.0 not in s.pair.P for s in c.valid)  # 29/fev nunca entra
    s1983 = c.ag_years[1983]  # ano agrícola 1983/84 contém 29/02/1984
    assert s1983.P.shape == (365,) and s1983.P[C.md_to_t(2, 28)] == P["1984-02-28"]
    assert s1983.P[C.md_to_t(3, 1)] == P["1984-03-01"]


def test_test_period_cannot_leak_into_construction(climate):
    c = O.build_scenarios(climate, "construction")
    last = c.scenarios[-1].pair
    P = climate["PRECTOTCORR"]
    assert last.P[-1] == P["2010-08-31"]  # último dia da construção
    # a série entregue à construção termina em 31/08/2010: nada de setembro/2010 em diante
    assert max(a.y for a in c.ag_years.values()) == 2009


@pytest.mark.parametrize("variable", ["PRECTOTCORR", "T2M", "T2M_MAX", "T2M_MIN"])
def test_missing_in_any_variable_excludes_scenarios_for_everyone(climate, variable):
    clim = climate.copy()
    clim.loc["1995-03-10", variable] = np.nan  # cai no ano agrícola 1994/95
    c = O.build_scenarios(clim, "construction")
    bad = {s.label for s in c.scenarios if not s.valid}
    assert bad == {"1993/94+1994/95", "1994/95+1995/96"}
    assert len(c.valid) == 26
    assert c.missing_log() == [{"ag_year": 1994, variable: 1}]
    assert all("dados ausentes" in e["reason"] for e in c.excluded())


def test_missing_date_row_counts_as_missing(climate):
    clim = climate.drop(pd.Timestamp("1990-07-04"))
    c = O.build_scenarios(clim, "construction")
    assert len(c.valid) == 26 and c.missing_log()[0]["ag_year"] == 1989


def test_999_must_be_nan_before_scenarios(climate):
    """O contrato exige NaN (data.load_climate converte -999). NaN invalida o cenário."""
    clim = climate.copy()
    clim.loc["2001-12-25", "PRECTOTCORR"] = np.nan
    c = O.build_scenarios(clim, "construction")
    assert len(c.valid) == 26


def test_sbar_and_reference_exclude_invalid_years(climate):
    clim = climate.copy()
    clim.loc["1995-03-10", "T2M_MAX"] = np.nan
    c = O.build_scenarios(clim, "construction")
    ref, sbar = O.construction_climatology(c)
    assert 1994 not in ref.labels and ref.n_years == 28
    # s_barra = média por dia dos anos válidos, repetida nos dois anos
    good = [y for y in range(1981, 2010) if y != 1994]
    manual = np.mean(
        [
            clim["PRECTOTCORR"][f"{y}-09-01" : f"{y + 1}-08-31"]
            .pipe(lambda s: s[~((s.index.month == 2) & (s.index.day == 29))])
            .to_numpy()[40]
            for y in good
        ]
    )
    assert sbar.P[40] == pytest.approx(manual) and sbar.P[40 + 365] == pytest.approx(manual)


def test_construction_gate_gives_dados_insuficientes(cat, soil, params, climate):
    clim = climate.copy()
    for y in range(1983, 2013, 3):  # 10 anos separados: invalida 20 dos 28 pares
        if y < 2010:
            clim.loc[f"{y}-11-15", "PRECTOTCORR"] = np.nan
    res = O.run(clim, cat, soil, params, snapshot_id="s")
    n_valid = res.scenarios["construction"]["n_valid"]
    assert n_valid < 20
    assert res.status == "DADOS_INSUFICIENTES" and "dados insuficientes" in res.message
    assert res.policies == {} and res.alternatives == []


def test_test_gate_gives_dados_insuficientes(cat, soil, params, climate):
    clim = climate.copy()
    for y in range(2011, 2025, 2):
        clim.loc[f"{y}-11-15", "T2M_MIN"] = np.nan
    res = O.run(clim, cat, soil, params, snapshot_id="s")
    assert res.scenarios["test"]["n_valid"] < 10 and res.status == "DADOS_INSUFICIENTES"


# ------------------------------------------------------------------ run (clima sintético)
def test_run_basic_structure(synth_run, cat, soil):
    r = synth_run
    assert r.status == "OK" and r.n_plans == 576 and r.risk_limit_pct == 20
    assert set(r.policies) == {"P0", "P1", "P1b", "P2"}
    assert r.snapshot_id == "synthetic-test" and len(r.config_hash) == 64
    plans, _ = C.enumerate_plans(cat, soil)
    byid = C.index_plans(plans)
    for name in ("P0", "P1", "P1b", "P2"):
        pl = byid[tuple(r.policies[name]["plan"]["id"])]
        assert C.verify_plan(cat, soil, pl) == []  # M0 = 100%
    assert len(r.sensitivity) == 15
    assert [tuple(row["weights"]) for row in r.sensitivity] == list(O.WEIGHT_GRID)
    row = r.sensitivity[4]  # (0,50; 0,25; 0,25) = padrão
    assert (
        row["P1"] == r.policies["P1"]["plan"]["id"] and row["P2"] == r.policies["P2"]["plan"]["id"]
    )
    assert row["P1b"] == r.policies["P1b"]["plan"]["id"]


def test_policies_satisfy_optimality_invariants(synth_run):
    m = {n: synth_run.policies[n]["metrics_construction"] for n in ("P0", "P1", "P1b", "P2")}
    for n in ("P0", "P1", "P1b"):
        assert m["P2"]["max_regret"] <= m[n]["max_regret"] + 1e-12  # P2 minimiza o regret máximo
    for n in ("P0", "P1", "P2"):
        assert m["P1b"]["mean_L"] <= m[n]["mean_L"] + 1e-12  # P1b minimiza a perda média
    for n in ("P0", "P1b", "P2"):
        assert m["P1"]["L_sbar"] <= m[n]["L_sbar"] + 1e-12  # P1 minimiza L no clima médio
    assert all(v["max_regret"] >= 0 for v in m.values())


def test_run_matches_independent_bruteforce(synth_run, cat, soil, params, climate):
    """Recalcula tudo com laços Python sobre os cenários de construção e compara as escolhas."""
    plans, _ = C.enumerate_plans(cat, soil)
    cset = O.build_scenarios(climate, "construction")
    ref, sbar = O.construction_climatology(cset)
    prm = params.with_reference(ref)
    w = synth_run.weights
    L = {}
    for p in plans:
        L[p.id] = [
            sum(a * b for a, b in zip(w, I.loss_components(p, s.pair, prm), strict=True))
            for s in cset.valid
        ]
    oracle = [min(L[p.id][j] for p in plans) for j in range(len(cset.valid))]

    def r6(v):
        return round(v, 9)

    mmr = min(
        plans,
        key=lambda p: (
            r6(max(L[p.id][j] - oracle[j] for j in range(len(oracle)))),
            r6(sum(L[p.id][j] - oracle[j] for j in range(len(oracle))) / len(oracle)),
            p.id,
        ),
    )
    exp = min(
        plans,
        key=lambda p: (
            r6(sum(L[p.id]) / len(oracle)),
            r6(max(L[p.id][j] - oracle[j] for j in range(len(oracle)))),
            p.id,
        ),
    )
    assert list(mmr.id) == synth_run.policies["P2"]["plan"]["id"]
    assert list(exp.id) == synth_run.policies["P1b"]["plan"]["id"]
    lb = {
        p.id: sum(a * b for a, b in zip(w, I.loss_components(p, sbar, prm), strict=True))
        for p in plans
    }
    conv = min(plans, key=lambda p: (r6(lb[p.id]), r6(sum(L[p.id]) / len(oracle)), p.id))
    assert list(conv.id) == synth_run.policies["P1"]["plan"]["id"]


def test_run_pareto_consistent_with_objectives(synth_run):
    rows = synth_run.objectives
    ids = [tuple(r[0]) for r in rows]
    F = np.array([r[1:] for r in rows])
    front_ids = {tuple(x) for x in synth_run.pareto["front_ids"]}
    naive = {tuple(F[i].tolist()) for i in _naive_front(F)}
    assert {tuple(F[ids.index(i)].tolist()) for i in front_ids} == naive
    assert len(front_ids) == synth_run.pareto["n_front"]


def test_alternatives_are_distinct(synth_run, cat, soil):
    plans, _ = C.enumerate_plans(cat, soil)
    byid = C.index_plans(plans)
    alts = synth_run.alternatives
    sigs = [byid[tuple(a["plan_id"])].signature for a in alts]
    assert len(sigs) == len(set(sigs)) <= O.K_ALTERNATIVES
    assert alts[0]["reason"] == "policy"
    for a, b in itertools.combinations(sigs, 2):
        assert O.hamming(a, b) >= 1
    for i, a in enumerate(alts):
        if a["reason"] != "policy" and not a["hamming_relaxed"]:
            assert all(O.hamming(sigs[i], sigs[j]) >= O.MIN_HAMMING for j in range(i))
    labels = [lab for a in alts for lab in a["labels"]]
    assert sorted(labels) == ["P0", "P1", "P1b", "P2"]  # cada política aparece uma vez


def test_alternatives_never_pad_with_duplicates():
    def mk(sig, i):
        occ = tuple(
            C.Occupation(w, c, None, None, None, None) for w, c in zip(C.WINDOWS, sig, strict=True)
        )
        return C.Plan((i, *sig), occ)

    plans = [mk((0, 2, 0, 2), 0), mk((0, 2, 0, 2), 1), mk((1, 2, 1, 2), 2)]  # só 2 assinaturas
    F = np.array([[0.1, 0.1, 0.1, 0.1], [0.1, 0.1, 0.1, 0.1], [0.2, 0.2, 0.2, 0.2]])
    front, _ = O.pareto_front(F, [p.id for p in plans])
    alts = O.select_alternatives(plans, F, front, [("P1", 0)], (0.5, 0.25, 0.25), k=6, delta=2)
    sigs = [plans[a["plan_index"]].signature for a in alts]
    assert len(sigs) == len(set(sigs)) == 1  # (0,2,0,2)<->(1,2,1,2) tem Hamming 2 mas é dominado


def test_fallow_reporting(synth_run, cat):
    r = synth_run
    assert r.fallow == {
        "total": 576,
        "com_pousio": 176,
        "pousio_J2": 96,
        "pousio_J4": 96,
        "pousio_J2_e_J4": 16,
    }
    u = r.fallow_usage
    assert u["x_plans_with_fallow"] == 176 and u["grid_size"] == 15
    assert set(u["grid_selections_with_fallow"]) == {"P1", "P1b", "P2"}
    assert u["front_size"] == r.pareto["n_front"]


def test_run_is_deterministic_and_json_roundtrip(cat, soil, params, climate, synth_run):
    again = O.run(climate, cat, soil, params, snapshot_id="synthetic-test")
    assert again.result_hash() == synth_run.result_hash()
    back = O.RunResult.from_dict(json.loads(again.to_json()))
    assert back.result_hash() == synth_run.result_hash()
    payload = json.loads(synth_run.to_json())
    assert (
        payload["result_hash"] == synth_run.result_hash()
        and payload["snapshot_id"] == "synthetic-test"
    )


def test_meta_is_excluded_from_result_hash(synth_run):
    h = synth_run.result_hash()
    synth_run.meta["wall_seconds"] = 1.23
    assert synth_run.result_hash() == h
    synth_run.meta.clear()


def test_T2_test_years_never_enter_the_choice(cat, soil, params, climate, synth_run):
    """Perturbar/embaralhar os anos de TESTE não altera planos, grade, Pareto nem s_barra."""
    clim = climate.copy()
    rng = np.random.default_rng(99)
    mask = clim.index >= pd.Timestamp("2010-09-01")
    clim.loc[mask, "PRECTOTCORR"] = rng.permutation(clim.loc[mask, "PRECTOTCORR"].to_numpy()) * 3.0
    clim.loc[mask, "T2M_MAX"] = clim.loc[mask, "T2M_MAX"] + 6.0
    res = O.run(clim, cat, soil, params, snapshot_id="synthetic-test")
    for key in (
        "policies",
        "sensitivity",
        "objectives",
        "pareto",
        "alternatives",
        "climatology",
        "fallow_usage",
    ):
        assert getattr(res, key) == getattr(synth_run, key), key
    assert res.result_hash() == synth_run.result_hash()
    # sanidade: perturbar a CONSTRUÇÃO muda o resultado
    clim2 = climate.copy()
    m2 = clim2.index < pd.Timestamp("2010-09-01")
    clim2.loc[m2, "PRECTOTCORR"] = clim2.loc[m2, "PRECTOTCORR"] * 0.2
    assert O.run(clim2, cat, soil, params, snapshot_id="x").result_hash() != synth_run.result_hash()


def test_inviavel_result_and_no_relaxation(cat, soil, params, climate):
    zarc = {k: tuple(30 if v == 20 else v for v in row) for k, row in cat.zarc.items()}
    res = O.run(climate, replace(cat, zarc=zarc), soil, params, snapshot_id="s")
    assert res.status == "INVIAVEL" and res.risk_limit_pct == 20
    assert res.diagnosis["status"] == "INVIAVEL" and res.rejections_summary
    assert res.policies == {} and res.alternatives == [] and res.n_plans == 0


def test_p0_outside_zarc_leaves_regret_comparison(cat, params, climate):
    soil1 = C.load_soil(CONFIG, ad_class="AD1")
    res = O.run(climate, cat, soil1, params, snapshot_id="s")
    assert res.status == "OK" and res.n_plans == 169
    p0 = res.policies["P0"]
    assert p0["status"] == "INVIAVEL_SOB_ZARC" and "inviável sob ZARC" in p0["message"]
    assert "plan" not in p0 and p0["reasons"]
    assert all(res.policies[n]["status"] == "OK" for n in ("P1", "P1b", "P2"))
    assert all("P0" not in a["labels"] for a in res.alternatives)
    assert "P0" not in res.fallow_usage["policies_w_star"]


def test_config_hash_tracks_configuration(cat, soil, params, climate):
    a = O.run(climate, cat, soil, params, snapshot_id="s")
    b = O.run(climate, cat, soil, params, snapshot_id="s", weights=(0.25, 0.5, 0.25))
    c = O.run(climate, cat, C.load_soil(CONFIG, ad_class="AD5"), params, snapshot_id="s")
    assert len({a.config_hash, b.config_hash, c.config_hash}) == 3
    with pytest.raises(O.OptimizationError):
        O.run(climate, cat, soil, params, snapshot_id="s", weights=(0.5, 0.5, 0.5))
    assert O.run(climate, cat, soil, params, snapshot_id="other").config_hash == a.config_hash
    assert a.snapshot_id != "other"


def test_run_requires_expected_columns(cat, soil, params, climate):
    with pytest.raises(O.OptimizationError, match="colunas"):
        O.run(climate.drop(columns=["T2M_MIN"]), cat, soil, params, snapshot_id="s")


# ------------------------------------------------------------------ run real (snapshot versionado)
def test_real_snapshot_run(real, cat, soil):
    res, clim, snap, secs = real
    assert res.status == "OK" and res.snapshot_id == snap.name
    assert res.n_plans == 576
    assert res.scenarios["construction"]["n_valid"] == 28 and res.scenarios["test"]["n_valid"] == 14
    assert (
        res.scenarios["construction"]["excluded"] == [] and res.scenarios["test"]["excluded"] == []
    )
    assert clim.index.min() == pd.Timestamp("1981-01-01")
    assert res.policies["P0"]["status"] == "OK"  # plano atual padrão é viável em AD4 + argilosa
    plans, _ = C.enumerate_plans(cat, soil)
    byid = C.index_plans(plans)
    for n in ("P0", "P1", "P1b", "P2"):
        assert C.verify_plan(cat, soil, byid[tuple(res.policies[n]["plan"]["id"])]) == []
    assert secs < 60


def test_real_snapshot_is_deterministic(real, cat, soil, params):
    res, clim, snap, _ = real
    again = O.run(data.load_climate(snap), cat, soil, params, snapshot_id=snap.name)
    assert again.result_hash() == res.result_hash()
    assert again.to_dict(include_meta=False) == res.to_dict(include_meta=False)


def test_real_snapshot_optimality_invariants(real):
    res = real[0]
    m = {n: res.policies[n]["metrics_construction"] for n in ("P0", "P1", "P1b", "P2")}
    for n in ("P0", "P1", "P1b"):
        assert m["P2"]["max_regret"] <= m[n]["max_regret"] + 1e-12
    for n in ("P0", "P1", "P2"):
        assert m["P1b"]["mean_L"] <= m[n]["mean_L"] + 1e-12
    assert res.pareto["n_front"] >= 1 and len(res.alternatives) >= 2
    assert all(0 <= v <= 1 for row in res.objectives for v in row[1:])


def test_real_snapshot_T2_invariance(real, cat, soil, params):
    res, clim, snap, _ = real
    pert = clim.copy()
    m = pert.index >= pd.Timestamp("2010-09-01")
    pert.loc[m, "PRECTOTCORR"] = np.roll(pert.loc[m, "PRECTOTCORR"].to_numpy(), 4000)
    pert.loc[m, "T2M_MAX"] = pert.loc[m, "T2M_MAX"] + 5
    res2 = O.run(pert, cat, soil, params, snapshot_id=snap.name)
    assert res2.policies == res.policies and res2.sensitivity == res.sensitivity
    assert res2.climatology == res.climatology and res2.pareto == res.pareto
    assert res2.result_hash() == res.result_hash()
