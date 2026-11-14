# AI-assisted (OpenBot agent)
"""Vazamento (protocolo v1.1, T1/T2) e completude do snapshot real."""

from __future__ import annotations

from pathlib import Path

import numpy as np
import pandas as pd
from test_evaluation import TOY_PLANS, ToyLossModel, synthetic_climate

from fieldshift import evaluation as ev

ROOT = Path(__file__).resolve().parents[1]
TEST_START = pd.Timestamp("2010-09-01")


def _perturb_test_years(c: pd.DataFrame, seed: int) -> pd.DataFrame:
    c = c.copy()
    m = c.index >= TEST_START
    rng = np.random.default_rng(seed)
    c.loc[m, "PRECTOTCORR"] = rng.permutation(c.loc[m, "PRECTOTCORR"].to_numpy()) * 1.7
    c.loc[m, "T2M_MAX"] = c.loc[m, "T2M_MAX"] + 4.0
    c.loc[m, "T2M_MIN"] = c.loc[m, "T2M_MIN"] + 4.0
    return c


def _construction_artifacts(climate):
    p = ev.EvalProtocol()
    split = ev.split_scenarios(climate, p)
    model = ToyLossModel()
    params = model.fit(split.construction)
    sbar = ev.mean_climate(split.construction_years)
    out = {
        "params": ev.sha256_json(params),
        "sbar": ev._array_hash(sbar.precip, sbar.tmax, sbar.tmin),
        "p25": ev.dry_threshold(split.construction_years, p),
    }
    ids = [x.id for x in TOY_PLANS]
    Gc = ev.component_matrix(TOY_PLANS, split.construction, model, params)
    Gb = ev.component_matrix(TOY_PLANS, [sbar], model, params)[:, 0]
    for w in ev.weight_grid():
        sel = ev.reference_selection(ev.losses(Gc, w), ev.losses(Gb, w), ids)
        out[str(w)] = {k: ids[v] for k, v in sel.items()}
    return out, model


def test_T1_fit_only_sees_construction():
    c = synthetic_climate()
    _, model = _construction_artifacts(c)
    (labels,) = model.fit_calls
    assert len(labels) == 28
    assert all(int(lbl[-7:-3]) <= 2009 for lbl in labels)  # último ano agrícola <= 2009/10


def test_T2_changing_test_years_changes_no_choice_param_or_climatology():
    c = synthetic_climate()
    base, _ = _construction_artifacts(c)
    for seed in (1, 2, 3):
        pert, _ = _construction_artifacts(_perturb_test_years(c, seed))
        assert pert == base  # medianas, s̄, P25 e planos (P1, P1b, P2) nos 15 pesos


def test_T2_full_evaluation_choices_unchanged_but_test_metrics_change():
    c = synthetic_climate()
    p = ev.EvalProtocol(n_boot=300)
    reports = []
    for clim in (c, _perturb_test_years(c, 5)):
        art, _ = _construction_artifacts(clim)
        pp = {
            k: next(x for x in TOY_PLANS if x.id == v)
            for k, v in art[str(ev.DEFAULT_WEIGHTS)].items()
        }
        pp["P0"] = TOY_PLANS[0]
        reports.append(
            ev.evaluate_plans(
                plans=TOY_PLANS,
                policy_plans=pp,
                climate=clim,
                loss_model=ToyLossModel(),
                protocol=p,
                run_grid=False,
            )
        )
    a, b = reports
    assert a.params_hash == b.params_hash and a.mean_climate_hash == b.mean_climate_hash
    assert a.audit["reference_selection"] == b.audit["reference_selection"]
    assert a.policies["P2"].per_scenario_loss != b.policies["P2"].per_scenario_loss


# --- snapshot real -----------------------------------------------------------------


def _snapshot_dir() -> Path:
    root = ROOT / "data" / "snapshots"
    return root / (root / "current.txt").read_text().strip()


def test_snapshot_manifest_tmax_tmin_complete_and_split_full():
    from fieldshift import data

    snap = _snapshot_dir()
    manifest = data.verify_manifest(snap)
    for v in ("PRECTOTCORR", "T2M_MAX", "T2M_MIN"):
        assert manifest["missing_fill_counts"][v] == 0
    assert manifest["request"]["requested_latitude"] == -18.0
    assert manifest["request"]["requested_longitude"] == -50.625
    clim = data.load_climate(snap)
    assert not clim[["PRECTOTCORR", "T2M_MAX", "T2M_MIN"]].isna().any().any()
    split = ev.split_scenarios(clim, ev.EvalProtocol())
    assert len(split.construction) == 28 and len(split.test) == 14 and not split.excluded


def test_protocol_file_is_frozen_v11():
    import hashlib

    blob = (ROOT / "docs" / "EVALUATION_PROTOCOL.md").read_bytes()
    assert hashlib.sha256(blob).hexdigest() == ev.PROTOCOL_SHA256


# --- motor real (Optimization) -------------------------------------------------------


def _real_setup():
    from fieldshift import catalog as cat
    from fieldshift import data, indicators

    snap = _snapshot_dir()
    clim = data.load_climate(snap)
    config = ROOT / "config"
    catalog = cat.load_catalog(config)
    soil = cat.load_soil(config)
    params = indicators.load_indicator_params(config, catalog)
    return clim, catalog, soil, params, snap.name


def test_T2_real_engine_choices_unchanged_when_test_years_change():
    from fieldshift import optimization as opt

    clim, catalog, soil, params, sid = _real_setup()
    a = opt.run(clim, catalog, soil, params, snapshot_id=sid)
    b = opt.run(_perturb_test_years(clim, 11), catalog, soil, params, snapshot_id=sid)
    assert a.status == b.status == "OK"
    for pol in ("P0", "P1", "P1b", "P2"):
        assert a.policies[pol].get("plan") == b.policies[pol].get("plan")
    assert a.climatology == b.climatology  # medianas de e3 (precip) e s̄
    assert a.sensitivity == b.sensitivity  # 15 pesos
    assert a.pareto == b.pareto


def test_T7_real_evaluation_parity_audit_and_determinism():
    from fieldshift import catalog as cat
    from fieldshift import optimization as opt

    clim, catalog, soil, params, sid = _real_setup()
    rr = opt.run(clim, catalog, soil, params, snapshot_id=sid)
    plans, _ = cat.enumerate_plans(catalog, soil, 0.20)
    proto = ev.EvalProtocol(n_boot=500)

    def once():
        return ev.evaluate(
            rr,
            clim,
            proto,
            plans=plans,
            loss_model=ev.IndicatorLossModel(params),
            check_plan=lambda p: cat.verify_plan(catalog, soil, p),
            optimizer_params_hash=rr.climatology["precip_sha256"],
            run_grid=False,
        )

    r1, r2 = once(), once()
    assert r1.result_hash() == r2.result_hash()
    assert r1.M0_feasibility == 1.0
    assert r1.audit["params_hash_match"] is True
    assert all(r1.audit["optimizer_matches_reference"].values())
    assert ev.sbar_sha256_like_optimizer(clim, proto) == rr.climatology["sbar_sha256"]
    assert r1.to_markdown().startswith(f"**DESFECHO: {r1.outcome}**")
