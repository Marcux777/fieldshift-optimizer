# AI-assisted (OpenBot agent)
"""Testes de fieldshift.data: offline, com o snapshot real e com casos adversos."""

from __future__ import annotations

import gzip
import json
import socket
from datetime import UTC, datetime
from pathlib import Path

import numpy as np
import pandas as pd
import pytest

from fieldshift import data

REPO = Path(__file__).resolve().parents[1]
SNAPSHOTS = REPO / "data" / "snapshots"
URL = "https://power.larc.nasa.gov/api/temporal/daily/point?test=1"
PARAMS = {"community": "AG", "latitude": -18.0, "longitude": -50.625, "format": "JSON"}
WHEN = datetime(2026, 11, 14, 12, 0, tzinfo=UTC)


# ------------------------------------------------------------------ helpers


def make_raw(
    n_days: int = 40,
    *,
    overrides: dict[tuple[str, str], float] | None = None,
    drop_dates: tuple[str, ...] = (),
    units: dict[str, str] | None = None,
    fill: float = -999.0,
    times: dict | None = None,
    drop_param: str | None = None,
) -> bytes:
    """Resposta sintética no formato do POWER (SÓ para testes adversos, nunca para o app)."""
    dates = [d.strftime("%Y%m%d") for d in pd.date_range("2000-01-01", periods=n_days)]
    dates = [d for d in dates if d not in drop_dates]
    rng = np.random.default_rng(0)
    params: dict[str, dict[str, float]] = {
        "PRECTOTCORR": {d: round(float(rng.uniform(0, 20)), 2) for d in dates},
        "T2M": {d: round(float(rng.uniform(18, 26)), 2) for d in dates},
        "T2M_MAX": {d: round(float(rng.uniform(26, 34)), 2) for d in dates},
        "T2M_MIN": {d: round(float(rng.uniform(12, 18)), 2) for d in dates},
    }
    for (col, date), value in (overrides or {}).items():
        params[col][date] = value
    if drop_param:
        params.pop(drop_param)
    doc = {
        "type": "Feature",
        "geometry": {"type": "Point", "coordinates": [-50.625, -18.0, 625.31]},
        "properties": {"parameter": params},
        "header": {
            "title": "NASA/POWER Source Native Resolution Daily Data",
            "api": {"version": "v2.10.3", "name": "POWER Daily API"},
            "sources": ["MERRA2", "POWER"],
            "fill_value": fill,
            "time_standard": "LST",
            "start": dates[0],
            "end": dates[-1],
        },
        "messages": [],
        "parameters": {k: {"units": v} for k, v in (units or data.EXPECTED_UNITS).items()},
        "times": times or {"data": 0.3, "process": 0.01},
    }
    return json.dumps(doc).encode()


def build(tmp_path: Path, raw: bytes | None = None) -> Path:
    return data.write_snapshot(
        raw if raw is not None else make_raw(),
        tmp_path,
        request_url=URL,
        request_params=PARAMS,
        collected_utc=WHEN,
    )


def rewrite_consistent(snap: Path, parquet_df: pd.DataFrame) -> None:
    """Troca o parquet e REAJUSTA manifesto e SHA256SUMS: simula adulteração 'competente'."""
    parquet_df.reset_index().to_parquet(snap / data.PARQUET_NAME, index=False)
    m = json.loads((snap / data.MANIFEST_NAME).read_text())
    m["files"]["parquet"]["sha256"] = data.sha256_file(snap / data.PARQUET_NAME)
    (snap / data.MANIFEST_NAME).write_text(json.dumps(m))
    sums = "".join(
        f"{data.sha256_file(snap / n)}  {n}\n"
        for n in (data.RAW_NAME, data.PARQUET_NAME, data.MANIFEST_NAME)
    )
    (snap / data.SUMS_NAME).write_text(sums)


# ------------------------------------------------------------------ snapshot real


@pytest.fixture(scope="module")
def real_snapshot() -> Path:
    return data.latest_snapshot(SNAPSHOTS)


def test_real_manifest_verifies_and_has_provenance(real_snapshot: Path) -> None:
    m = data.verify_manifest(real_snapshot)
    assert m["snapshot_id"] == real_snapshot.name
    assert m["source"]["api_version"] == "v2.10.3"
    assert m["source"]["community"] == "AG"
    assert m["request"]["returned_geometry"]["latitude"] == -18.0
    assert m["request"]["returned_geometry"]["longitude"] == -50.625
    assert m["request"]["parameters"] == list(data.COLUMNS)
    assert m["fill_value"] == -999.0
    assert m["rows"] == 16436 and m["date_min"] == "1981-01-01" and m["date_max"] == "2025-12-31"
    assert m["units"] == data.EXPECTED_UNITS
    assert "Prediction Of Worldwide Energy Resources" in m["citation"]["reference"]
    assert "Daily" in m["citation"]["data_reference"] and "2026/" in m["citation"]["data_reference"]
    assert len(m["files"]["raw"]["sha256"]) == 64 and len(m["files"]["parquet"]["sha256"]) == 64


def test_real_load_climate_shape_and_no_zero_fill(real_snapshot: Path) -> None:
    df = data.load_climate(real_snapshot)
    assert list(df.columns) == list(data.COLUMNS)
    assert df.index.name == "date" and isinstance(df.index, pd.DatetimeIndex)
    assert len(df) == 16436
    assert df.index[0] == pd.Timestamp("1981-01-01") and df.index[-1] == pd.Timestamp("2025-12-31")
    assert df.index.is_unique and df.index.is_monotonic_increasing
    assert not df.isna().any().any()  # este snapshot: nenhum -999 nas 4 variáveis
    assert not (df == -999).any().any()
    assert (df["PRECTOTCORR"] >= 0).all() and (df["T2M_MIN"] <= df["T2M_MAX"]).all()


def test_real_matches_raw_response_independently(real_snapshot: Path) -> None:
    """Leitura independente do JSON bruto (sem passar por data.py) bate com load_climate."""
    raw = json.loads(gzip.decompress((real_snapshot / data.RAW_NAME).read_bytes()))
    p = raw["properties"]["parameter"]
    df = data.load_climate(real_snapshot)
    for col in data.COLUMNS:
        for d in ("19810101", "20000229", "20251231"):
            assert df.loc[pd.Timestamp(d), col] == pytest.approx(p[col][d])
    assert raw["header"]["fill_value"] == -999.0


def test_real_quality_report_clean(real_snapshot: Path) -> None:
    m = data.read_manifest(real_snapshot)
    rep = data.quality_report(data.load_climate(real_snapshot), m)
    assert rep["ok"] is True
    assert rep["rows"] == rep["expected_calendar_days"] == 16436
    assert rep["calendar_gaps"] == 0 and rep["duplicate_dates"] == 0
    assert all(v == 0 for v in rep["missing_by_column"].values())
    assert rep["units_match_expected"] is True
    assert rep["leap_days_present"] == 11  # 1984..2024 (11 anos bissextos em 1981-2025)


def test_load_climate_is_offline(real_snapshot: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    def boom(*_a, **_k):
        raise AssertionError("acesso à rede em load_climate")

    monkeypatch.setattr(socket.socket, "connect", boom)
    assert len(data.load_climate(real_snapshot)) == 16436


# ------------------------------------------------------------------ casos adversos


def test_fill_value_becomes_nan_never_zero(tmp_path: Path) -> None:
    raw = make_raw(overrides={("PRECTOTCORR", "20000110"): -999.0, ("T2M", "20000111"): -999.0})
    snap = build(tmp_path, raw)
    df = data.load_climate(snap)
    assert np.isnan(df.loc["2000-01-10", "PRECTOTCORR"])
    assert np.isnan(df.loc["2000-01-11", "T2M"])
    assert not (df == 0).any().any()
    assert not (df == -999).any().any()
    m = data.read_manifest(snap)
    assert m["missing_fill_counts"] == {"PRECTOTCORR": 1, "T2M": 1, "T2M_MAX": 0, "T2M_MIN": 0}
    rep = data.quality_report(df, m)
    assert rep["ok"] is False
    assert rep["missing_by_column"]["PRECTOTCORR"] == 1
    assert rep["first_missing_dates_by_column"]["PRECTOTCORR"] == ["2000-01-10"]


def test_residual_fill_in_parquet_is_converted_not_kept(tmp_path: Path) -> None:
    snap = build(tmp_path)
    df = pd.read_parquet(snap / data.PARQUET_NAME).set_index("date")
    df.iloc[3, 0] = -999.0
    rewrite_consistent(snap, df)
    out = data.load_climate(snap, verify=False)  # sem verificação de hash: defesa em profundidade
    assert np.isnan(out.iloc[3, 0])
    with pytest.raises(data.SnapshotError):  # com verificação, o parquet não bate com o bruto
        data.load_climate(snap)


def test_duplicate_date_in_raw_json_is_rejected(tmp_path: Path) -> None:
    raw = make_raw().decode().replace('"20000105": ', '"20000104": ', 1).encode()
    with pytest.raises(data.SnapshotError, match="duplicada"):
        build(tmp_path, raw)


def test_duplicate_date_in_parquet_is_rejected(tmp_path: Path) -> None:
    snap = build(tmp_path)
    df = pd.read_parquet(snap / data.PARQUET_NAME).set_index("date")
    df = pd.concat([df, df.iloc[[5]]]).sort_index()  # data duplicada
    rewrite_consistent(snap, df)
    with pytest.raises(data.SnapshotError, match="duplicada"):
        data.load_climate(snap, verify=False)
    with pytest.raises(data.SnapshotError):
        data.load_climate(snap)


@pytest.mark.parametrize("target", ["parquet", "raw", "manifest_hash", "sums"])
def test_altered_hash_fails_loudly(tmp_path: Path, target: str) -> None:
    snap = build(tmp_path)
    data.verify_manifest(snap)  # íntegro antes
    if target == "parquet":
        b = bytearray((snap / data.PARQUET_NAME).read_bytes())
        b[len(b) // 2] ^= 0xFF
        (snap / data.PARQUET_NAME).write_bytes(bytes(b))
    elif target == "raw":
        raw = gzip.decompress((snap / data.RAW_NAME).read_bytes()).replace(b"20.", b"21.", 1)
        (snap / data.RAW_NAME).write_bytes(gzip.compress(raw))
    elif target == "manifest_hash":
        m = json.loads((snap / data.MANIFEST_NAME).read_text())
        m["files"]["parquet"]["sha256"] = "0" * 64
        (snap / data.MANIFEST_NAME).write_text(json.dumps(m))
    else:
        (snap / data.SUMS_NAME).write_text("0" * 64 + "  daily.parquet\n")
    with pytest.raises(data.SnapshotError):
        data.verify_manifest(snap)
    with pytest.raises(data.SnapshotError):
        data.load_climate(snap)


def test_altered_value_with_consistent_hashes_is_still_caught(tmp_path: Path) -> None:
    """Adulteração que reajusta hashes do parquet: o cruzamento com o bruto a denuncia."""
    snap = build(tmp_path)
    df = pd.read_parquet(snap / data.PARQUET_NAME).set_index("date")
    df.iloc[0, 0] += 1.0
    rewrite_consistent(snap, df)
    with pytest.raises(data.SnapshotError, match="não corresponde"):
        data.verify_manifest(snap)


def test_missing_files_and_manifest(tmp_path: Path) -> None:
    snap = build(tmp_path)
    (snap / data.PARQUET_NAME).unlink()
    with pytest.raises(data.SnapshotError, match="ausente"):
        data.verify_manifest(snap)
    (snap / data.MANIFEST_NAME).unlink()
    with pytest.raises(data.SnapshotError, match="manifest.json ausente"):
        data.verify_manifest(snap)


@pytest.mark.parametrize(
    "kwargs,match",
    [
        ({"units": {**data.EXPECTED_UNITS, "PRECTOTCORR": "mm/hour"}}, "unidade inesperada"),
        ({"fill": -9999.0}, "fill_value"),
        ({"drop_param": "T2M_MIN"}, "ausentes"),
        ({"overrides": {("PRECTOTCORR", "20000105"): -3.0}}, "negativa"),
        ({"overrides": {("T2M_MIN", "20000105"): 40.0}}, "T2M_MIN > T2M_MAX"),
    ],
)
def test_contract_violations_fail(tmp_path: Path, kwargs: dict, match: str) -> None:
    with pytest.raises(data.SnapshotError, match=match):
        build(tmp_path, make_raw(**kwargs))
    assert not any(tmp_path.iterdir())  # nada parcial fica para trás


def test_invalid_json_rejected(tmp_path: Path) -> None:
    with pytest.raises(data.SnapshotError, match="JSON"):
        build(tmp_path, b"<html>502 Bad Gateway</html>")


def test_calendar_gap_is_reported(tmp_path: Path) -> None:
    snap = build(tmp_path, make_raw(drop_dates=("20000110", "20000111")))
    rep = data.quality_report(data.load_climate(snap), data.read_manifest(snap))
    assert rep["calendar_gaps"] == 2 and rep["calendar_gap_dates"] == ["2000-01-10", "2000-01-11"]
    assert rep["ok"] is False


# ------------------------------------------------------------------ idempotência


def test_write_snapshot_is_idempotent_despite_volatile_times(tmp_path: Path) -> None:
    a = build(tmp_path, make_raw(times={"data": 0.34, "process": 0.01}))
    before = {p.name: data.sha256_file(p) for p in a.rglob("*") if p.is_file()}
    b = build(tmp_path, make_raw(times={"data": 9.99, "process": 0.5}))  # bytes brutos diferem
    assert a == b
    assert sorted(p.name for p in tmp_path.iterdir()) == [a.name]  # sem duplicata nem .tmp
    assert before == {p.name: data.sha256_file(p) for p in b.rglob("*") if p.is_file()}


def test_different_data_gives_different_id(tmp_path: Path) -> None:
    a = build(tmp_path, make_raw())
    b = build(tmp_path, make_raw(overrides={("T2M", "20000110"): 21.5}))
    assert a != b and a.name != b.name


def test_gzip_is_deterministic() -> None:
    assert data.gzip_deterministic(b"abc" * 1000) == data.gzip_deterministic(b"abc" * 1000)


def test_latest_snapshot_rules(tmp_path: Path) -> None:
    with pytest.raises(data.SnapshotError):
        data.latest_snapshot(tmp_path)  # vazio e sem current.txt
    snap = build(tmp_path)
    assert data.latest_snapshot(tmp_path) == snap
    (tmp_path / "current.txt").write_text("nao-existe\n")
    with pytest.raises(data.SnapshotError, match="inexistente"):
        data.latest_snapshot(tmp_path)
