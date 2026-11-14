# AI-assisted (OpenBot agent)
from pathlib import Path

import pytest

from fieldshift import catalog, data, optimization, pipeline


@pytest.mark.parametrize("weights", [(0.5, 0.5, 0.5), (float('nan'), 0.5, 0.5), (-1, 1, 1)])
def test_invalid_weights_fail_before_data_load(weights, monkeypatch):
    def forbidden(*args):
        pytest.fail("Invalid inputs must not touch the snapshot")
    monkeypatch.setattr(data, "load_climate", forbidden)
    with pytest.raises(pipeline.PipelineError, match="Weights"):
        pipeline.run_all(pipeline.Inputs(weights=weights))


def test_missing_snapshot_is_not_replaced(tmp_path):
    with pytest.raises(pipeline.PipelineError, match="Cannot complete"):
        pipeline.run_all(pipeline.Inputs(snapshot_dir=tmp_path / "missing"))


def test_insufficient_climate_is_explicit(monkeypatch):
    real_load = data.load_climate
    monkeypatch.setattr(data, "load_climate", lambda path: real_load(path).iloc[:10])
    rr, report = pipeline.run_all(pipeline.Inputs())
    assert rr.status == report.outcome == "DADOS_INSUFICIENTES"
    assert not report.policies


def test_empty_feasible_set_is_explicit(monkeypatch):
    monkeypatch.setattr(catalog, "enumerate_plans", lambda *a, **k: ([], []))
    monkeypatch.setattr(optimization, "enumerate_plans", lambda *a, **k: ([], []))
    rr, report = pipeline.run_all(pipeline.Inputs())
    assert rr.status == report.outcome == "INVIAVEL"
    assert rr.n_plans == 0
    assert not report.policies


def test_real_snapshot_complete_offline(monkeypatch):
    import socket
    def no_network(*args, **kwargs):
        raise AssertionError("Pipeline must run offline")
    monkeypatch.setattr(socket.socket, "connect", no_network)
    rr, report = pipeline.run_all(pipeline.Inputs())
    assert rr.status == "OK"
    assert rr.n_plans == 576
    assert len(report.construction) == 28 and len(report.test) == 14
    assert report.outcome == "NEGATIVO"
    assert report.M0_feasibility == 1.0
    assert all(report.policies[p].feasible for p in ("P0", "P1", "P1b", "P2"))
    assert report.policies['P2'].M1_max_regret > report.policies['P1b'].M1_max_regret
    assert len(rr.sensitivity) == len(report.sensitivity) == 15
    assert rr.snapshot_id.startswith("power-daily-")
    assert len(rr.config_hash) == 64
    assert Path(pipeline.ROOT / 'app.py').is_file()
