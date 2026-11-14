# AI-assisted (OpenBot agent)
"""Snapshot versionado da NASA POWER: escrita, verificação e leitura.

Este módulo é um adaptador de I/O de arquivos. Ele NÃO usa rede: o único código que acessa a
rede é ``scripts/collect_snapshot.py``. Regras:

* ``-999`` (fill value do POWER) vira ``NaN``, nunca 0 mm / 0 °C.
* Hash divergente, data duplicada, lacuna não declarada ou unidade inesperada falham alto.
* Sem fallback silencioso e sem dado sintético.

Layout de ``data/snapshots/<snapshot_id>/``::

    raw/power_daily.json.gz   resposta bruta da API (gzip determinístico), fonte da verdade
    daily.parquet             derivado: 1 linha por dia, -999 -> NaN
    manifest.json             proveniência, hashes, citação
    SHA256SUMS                hashes dos três arquivos acima (conferência por ``sha256sum -c``)
"""

from __future__ import annotations

import gzip
import hashlib
import io
import json
import os
import platform
import shutil
from collections.abc import Mapping
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

import numpy as np
import pandas as pd

SOURCE_SERVICE = "NASA POWER Daily API"
FILL_VALUE = -999.0
COLUMNS: tuple[str, ...] = ("PRECTOTCORR", "T2M", "T2M_MAX", "T2M_MIN")
EXPECTED_UNITS: dict[str, str] = {
    "PRECTOTCORR": "mm/day",
    "T2M": "C",
    "T2M_MAX": "C",
    "T2M_MIN": "C",
}
RAW_NAME = "raw/power_daily.json.gz"
PARQUET_NAME = "daily.parquet"
MANIFEST_NAME = "manifest.json"
SUMS_NAME = "SHA256SUMS"
MANIFEST_SCHEMA = 1

POWER_REFERENCE = (
    "The data was obtained from National Aeronautics and Space Administration (NASA) Langley "
    "Research Center's Prediction Of Worldwide Energy Resources (POWER) project funded through "
    "the NASA Earth Science Division."
)
POWER_DATA_REFERENCE_TEMPLATE = (
    "The data was obtained from the POWER Project's Daily {version} version on {date}."
)
POWER_REFERENCING_URL = "https://power.larc.nasa.gov/docs/referencing/"


class SnapshotError(ValueError):
    """Snapshot inconsistente, adulterado ou fora do contrato."""


# --------------------------------------------------------------------------- utilidades


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def gzip_deterministic(data: bytes) -> bytes:
    """gzip sem timestamp nem nome de arquivo: mesmo conteúdo, mesmos bytes."""
    buf = io.BytesIO()
    with gzip.GzipFile(fileobj=buf, mode="wb", compresslevel=9, mtime=0, filename="") as gz:
        gz.write(data)
    return buf.getvalue()


def _no_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    out: dict[str, Any] = {}
    for key, value in pairs:
        if key in out:
            raise SnapshotError(f"chave duplicada na resposta bruta: {key!r}")
        out[key] = value
    return out


# --------------------------------------------------------------------------- parsing

# Campos da resposta que variam entre requisições idênticas (tempo de processamento do servidor).
VOLATILE_KEYS: tuple[str, ...] = ("times",)


def content_sha256(raw: bytes) -> str:
    """Hash do CONTEÚDO da resposta, sem campos voláteis (``times``) e com chaves ordenadas.

    O corpo bruto não é estável: ``times`` traz o tempo de processamento do servidor e muda a
    cada chamada (verificado em 14/11: mesma requisição, dados idênticos, bytes diferentes). O
    id do snapshot deriva deste hash, para a coleta repetida dar o mesmo id.
    """
    try:
        doc = json.loads(raw, object_pairs_hook=_no_duplicate_keys)
    except json.JSONDecodeError as exc:
        raise SnapshotError(f"resposta bruta não é JSON válido: {exc}") from exc
    for key in VOLATILE_KEYS:
        doc.pop(key, None)
    canon = json.dumps(doc, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return sha256_bytes(canon.encode())


@dataclass(frozen=True)
class ParsedResponse:
    frame: pd.DataFrame  # índice 'date', colunas COLUMNS, -999 => NaN
    api_version: str
    sources: list[str]
    time_standard: str
    fill_value: float
    geometry: list[float]  # [lon, lat, elevação]
    units: dict[str, str]
    start: str
    end: str
    missing_fill_counts: dict[str, int]


def parse_power_response(raw: bytes) -> ParsedResponse:
    """Converte a resposta JSON bruta do POWER em DataFrame, validando o contrato."""
    try:
        doc = json.loads(raw, object_pairs_hook=_no_duplicate_keys)
    except json.JSONDecodeError as exc:
        raise SnapshotError(f"resposta bruta não é JSON válido: {exc}") from exc
    try:
        header = doc["header"]
        params = doc["properties"]["parameter"]
        geometry = [float(x) for x in doc["geometry"]["coordinates"]]
        units = {k: v["units"] for k, v in doc["parameters"].items()}
        fill = float(header["fill_value"])
        api_version = str(header["api"]["version"])
    except (KeyError, TypeError, ValueError) as exc:
        raise SnapshotError(f"resposta bruta fora do formato esperado do POWER: {exc!r}") from exc

    if fill != FILL_VALUE:
        raise SnapshotError(f"fill_value inesperado: {fill} (esperado {FILL_VALUE})")
    missing_cols = [c for c in COLUMNS if c not in params]
    if missing_cols:
        raise SnapshotError(f"parâmetros ausentes na resposta: {missing_cols}")
    extra = sorted(set(params) - set(COLUMNS))
    if extra:
        raise SnapshotError(f"parâmetros não previstos na resposta: {extra}")
    for col, unit in EXPECTED_UNITS.items():
        if units.get(col) != unit:
            raise SnapshotError(f"unidade inesperada para {col}: {units.get(col)!r} != {unit!r}")

    date_sets = {c: set(params[c]) for c in COLUMNS}
    ref = date_sets[COLUMNS[0]]
    for col, keys in date_sets.items():
        if keys != ref:
            raise SnapshotError(f"{col} tem datas diferentes de {COLUMNS[0]}")

    data: dict[str, np.ndarray] = {}
    fill_counts: dict[str, int] = {}
    for col in COLUMNS:
        keys = sorted(params[col])
        values = np.array(
            [np.nan if params[col][k] is None else float(params[col][k]) for k in keys],
            dtype="float64",
        )
        is_fill = values == FILL_VALUE
        fill_counts[col] = int(is_fill.sum())
        values[is_fill] = np.nan  # nunca 0
        data[col] = values
        if col == COLUMNS[0]:
            index = pd.to_datetime(keys, format="%Y%m%d", errors="raise")
    frame = pd.DataFrame(data, index=pd.DatetimeIndex(index, name="date"))
    return ParsedResponse(
        frame=frame,
        api_version=api_version,
        sources=list(header.get("sources", [])),
        time_standard=str(header.get("time_standard", "")),
        fill_value=fill,
        geometry=geometry,
        units=units,
        start=str(header.get("start", "")),
        end=str(header.get("end", "")),
        missing_fill_counts=fill_counts,
    )


# --------------------------------------------------------------------------- qualidade


def validate_climate_frame(df: pd.DataFrame) -> None:
    """Invariantes duras. Levanta SnapshotError no primeiro problema."""
    missing = [c for c in COLUMNS if c not in df.columns]
    if missing:
        raise SnapshotError(f"colunas ausentes: {missing}")
    if not isinstance(df.index, pd.DatetimeIndex):
        raise SnapshotError("o índice precisa ser DatetimeIndex")
    dup = df.index[df.index.duplicated()]
    if len(dup):
        raise SnapshotError(f"{len(dup)} data(s) duplicada(s), ex.: {dup[0].date()}")
    if not df.index.is_monotonic_increasing:
        raise SnapshotError("índice de datas fora de ordem")
    if (df[list(COLUMNS)] == FILL_VALUE).any().any():
        raise SnapshotError("há -999 não convertido em NaN")
    if (df["PRECTOTCORR"] < 0).any():
        raise SnapshotError("PRECTOTCORR negativa")
    if (df["T2M_MIN"] > df["T2M_MAX"]).any():
        raise SnapshotError("T2M_MIN > T2M_MAX em algum dia")


def quality_report(df: pd.DataFrame, manifest: Mapping[str, Any] | None = None) -> dict[str, Any]:
    """Relatório de qualidade: contagens, ausências, duplicatas, lacunas e unidades."""
    idx = df.index
    full = pd.date_range(idx.min(), idx.max(), freq="D") if len(idx) else idx
    gaps = full.difference(idx)
    missing = {c: int(df[c].isna().sum()) for c in COLUMNS if c in df.columns}
    dates_missing = {
        c: [d.strftime("%Y-%m-%d") for d in df.index[df[c].isna()][:5]] for c in missing
    }
    ranges = {
        c: [float(np.nanmin(df[c])), float(np.nanmax(df[c]))]
        for c in COLUMNS
        if c in df.columns and df[c].notna().any()
    }
    report: dict[str, Any] = {
        "rows": len(df),
        "date_min": idx.min().strftime("%Y-%m-%d") if len(idx) else None,
        "date_max": idx.max().strftime("%Y-%m-%d") if len(idx) else None,
        "expected_calendar_days": len(full),
        "calendar_gaps": len(gaps),
        "calendar_gap_dates": [d.strftime("%Y-%m-%d") for d in gaps[:10]],
        "duplicate_dates": int(idx.duplicated().sum()),
        "monotonic": bool(idx.is_monotonic_increasing),
        "missing_by_column": missing,
        "first_missing_dates_by_column": dates_missing,
        "value_ranges": ranges,
        "units": dict(manifest["units"]) if manifest and "units" in manifest else EXPECTED_UNITS,
        "units_match_expected": (
            dict(manifest["units"]) == EXPECTED_UNITS if manifest and "units" in manifest else None
        ),
        "leap_days_present": int(((idx.month == 2) & (idx.day == 29)).sum()),
    }
    report["ok"] = (
        report["calendar_gaps"] == 0
        and report["duplicate_dates"] == 0
        and report["monotonic"]
        and sum(missing.values()) == 0
        and report["units_match_expected"] is not False
    )
    return report


# --------------------------------------------------------------------------- escrita


def _frame_to_parquet_bytes(frame: pd.DataFrame) -> bytes:
    buf = io.BytesIO()
    frame.reset_index().to_parquet(buf, engine="pyarrow", compression="zstd", index=False)
    return buf.getvalue()


def _tool_versions() -> dict[str, str]:
    import pyarrow

    return {
        "python": platform.python_version(),
        "pandas": pd.__version__,
        "numpy": np.__version__,
        "pyarrow": pyarrow.__version__,
    }


def snapshot_id_for(parsed: ParsedResponse, content_sha: str) -> str:
    lon, lat = parsed.geometry[0], parsed.geometry[1]
    return f"power-daily-{parsed.api_version.lstrip('v')}-lat{lat:g}_lon{lon:g}-{content_sha[:12]}"


def write_snapshot(
    raw: bytes,
    snapshots_root: Path,
    *,
    request_url: str,
    request_params: Mapping[str, Any],
    collected_utc: datetime | None = None,
) -> Path:
    """Valida a resposta bruta e grava o snapshot de forma idempotente.

    Mesmo conteúdo => mesmo ``snapshot_id`` => nada é regravado (retorna o diretório existente,
    após verificá-lo). Escreve em diretório temporário e só então renomeia.
    """
    parsed = parse_power_response(raw)
    validate_climate_frame(parsed.frame)
    raw_sha = sha256_bytes(raw)
    content_sha = content_sha256(raw)
    sid = snapshot_id_for(parsed, content_sha)
    snapshots_root = Path(snapshots_root)
    final = snapshots_root / sid
    if final.exists():
        verify_manifest(final)
        return final

    collected = (collected_utc or datetime.now(UTC)).astimezone(UTC)
    tmp = snapshots_root / f".tmp-{sid}-{os.getpid()}"
    if tmp.exists():
        shutil.rmtree(tmp)
    (tmp / "raw").mkdir(parents=True)
    try:
        raw_gz = gzip_deterministic(raw)
        (tmp / RAW_NAME).write_bytes(raw_gz)
        parquet = _frame_to_parquet_bytes(parsed.frame)
        (tmp / PARQUET_NAME).write_bytes(parquet)

        frame = parsed.frame
        manifest: dict[str, Any] = {
            "manifest_schema": MANIFEST_SCHEMA,
            "snapshot_id": sid,
            "collected_utc": collected.strftime("%Y-%m-%dT%H:%M:%SZ"),
            "source": {
                "service": SOURCE_SERVICE,
                "api_version": parsed.api_version,
                "sources": parsed.sources,
                "community": request_params.get("community"),
                "time_standard": parsed.time_standard,
            },
            "request": {
                "url": request_url,
                "parameters": list(COLUMNS),
                "requested_latitude": request_params.get("latitude"),
                "requested_longitude": request_params.get("longitude"),
                "returned_geometry": {
                    "longitude": parsed.geometry[0],
                    "latitude": parsed.geometry[1],
                    "elevation_m": parsed.geometry[2] if len(parsed.geometry) > 2 else None,
                },
                "start": parsed.start,
                "end": parsed.end,
                "format": request_params.get("format"),
            },
            "fill_value": parsed.fill_value,
            "units": parsed.units,
            "rows": len(frame),
            "date_min": frame.index.min().strftime("%Y-%m-%d"),
            "date_max": frame.index.max().strftime("%Y-%m-%d"),
            "missing_fill_counts": parsed.missing_fill_counts,
            "known_gaps": [],
            "files": {
                "raw": {
                    "path": RAW_NAME,
                    "sha256": sha256_bytes(raw_gz),
                    "bytes": len(raw_gz),
                    "uncompressed_sha256": raw_sha,
                    "uncompressed_bytes": len(raw),
                    "content_sha256": content_sha,
                    "content_hash_note": "sha256 do JSON canônico sem o campo volátil 'times'",
                },
                "parquet": {
                    "path": PARQUET_NAME,
                    "sha256": sha256_bytes(parquet),
                    "bytes": len(parquet),
                    "schema": {c: "float64" for c in COLUMNS},
                    "index": "date (datetime64[ns])",
                },
            },
            "tool_versions": _tool_versions(),
            "citation": {
                "reference": POWER_REFERENCE,
                "data_reference": POWER_DATA_REFERENCE_TEMPLATE.format(
                    version=parsed.api_version, date=collected.strftime("%Y/%m/%d")
                ),
                "referencing_guide": POWER_REFERENCING_URL,
                "note": (
                    "O POWER pede nome do serviço, versão e data de acesso. Texto de licença "
                    "formal não foi localizado na página de referência; não afirmar licença."
                ),
            },
        }
        manifest_bytes = (json.dumps(manifest, indent=2, ensure_ascii=False) + "\n").encode()
        (tmp / MANIFEST_NAME).write_bytes(manifest_bytes)
        sums = "".join(
            f"{sha256_file(tmp / name)}  {name}\n"
            for name in (RAW_NAME, PARQUET_NAME, MANIFEST_NAME)
        )
        (tmp / SUMS_NAME).write_text(sums)
        verify_manifest(tmp, check_dir_name=False)
        os.replace(tmp, final)
    except BaseException:
        shutil.rmtree(tmp, ignore_errors=True)
        raise
    return final


# --------------------------------------------------------------------------- leitura


def read_manifest(snapshot_dir: Path) -> dict[str, Any]:
    path = Path(snapshot_dir) / MANIFEST_NAME
    if not path.is_file():
        raise SnapshotError(f"manifest.json ausente em {snapshot_dir}")
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        raise SnapshotError(f"manifest.json inválido: {exc}") from exc


def _read_parquet(snapshot_dir: Path) -> pd.DataFrame:
    df = pd.read_parquet(Path(snapshot_dir) / PARQUET_NAME, engine="pyarrow")
    if "date" not in df.columns:
        raise SnapshotError("daily.parquet sem coluna 'date'")
    df["date"] = pd.to_datetime(df["date"])
    return df.set_index("date")


def verify_manifest(snapshot_dir: Path, *, check_dir_name: bool = True) -> dict[str, Any]:
    """Confere hashes e a consistência raw -> parquet. Levanta SnapshotError se algo divergir.

    Retorna o manifesto em caso de sucesso.
    """
    snapshot_dir = Path(snapshot_dir)
    manifest = read_manifest(snapshot_dir)
    if manifest.get("manifest_schema") != MANIFEST_SCHEMA:
        raise SnapshotError(f"manifest_schema inesperado: {manifest.get('manifest_schema')!r}")
    files = manifest.get("files", {})
    for key in ("raw", "parquet"):
        entry = files.get(key)
        if not entry:
            raise SnapshotError(f"manifesto sem entrada de arquivo '{key}'")
        path = snapshot_dir / entry["path"]
        if not path.is_file():
            raise SnapshotError(f"arquivo ausente: {entry['path']}")
        got = sha256_file(path)
        if got != entry["sha256"]:
            raise SnapshotError(
                f"hash divergente em {entry['path']}: manifesto={entry['sha256']} atual={got}"
            )
    raw_path = snapshot_dir / files["raw"]["path"]
    raw = gzip.decompress(raw_path.read_bytes())
    if sha256_bytes(raw) != files["raw"]["uncompressed_sha256"]:
        raise SnapshotError("hash da resposta bruta descomprimida diverge do manifesto")
    if check_dir_name and manifest["snapshot_id"] != snapshot_dir.name:
        raise SnapshotError(
            f"snapshot_id {manifest['snapshot_id']!r} diverge do diretório {snapshot_dir.name!r}"
        )
    if content_sha256(raw) != files["raw"]["content_sha256"]:
        raise SnapshotError("content_sha256 diverge do manifesto")
    if not manifest["snapshot_id"].endswith(files["raw"]["content_sha256"][:12]):
        raise SnapshotError("snapshot_id não deriva do hash do conteúdo")

    sums_path = snapshot_dir / SUMS_NAME
    if sums_path.is_file():
        for line in sums_path.read_text().splitlines():
            digest, _, name = line.partition("  ")
            if sha256_file(snapshot_dir / name) != digest:
                raise SnapshotError(f"SHA256SUMS diverge em {name}")

    # parquet precisa ser exatamente a derivação do bruto (-999 => NaN)
    parsed = parse_power_response(raw)
    derived = parsed.frame
    stored = _read_parquet(snapshot_dir)
    try:
        pd.testing.assert_frame_equal(
            stored[list(COLUMNS)], derived[list(COLUMNS)], check_freq=False
        )
    except AssertionError as exc:
        raise SnapshotError(f"daily.parquet não corresponde à resposta bruta: {exc}") from exc
    if len(derived) != manifest["rows"]:
        raise SnapshotError("número de linhas diverge do manifesto")
    return manifest


def load_climate(snapshot_dir: Path, *, verify: bool = True) -> pd.DataFrame:
    """Clima diário do snapshot: índice ``date``, colunas PRECTOTCORR, T2M, T2M_MAX, T2M_MIN.

    -999 => NaN, nunca 0. ``verify=True`` (padrão) confere hashes antes de ler e falha alto se
    divergirem. ``verify=False`` existe só para testes de invariantes.
    """
    snapshot_dir = Path(snapshot_dir)
    if verify:
        verify_manifest(snapshot_dir)
    df = _read_parquet(snapshot_dir)[list(COLUMNS)].astype("float64")
    df = df.mask(df == FILL_VALUE)  # defesa: qualquer -999 residual vira NaN
    validate_climate_frame(df)
    return df


def latest_snapshot(snapshots_root: Path) -> Path:
    """Snapshot vigente: o que ``current.txt`` aponta; sem ele, exige exatamente um snapshot."""
    root = Path(snapshots_root)
    current = root / "current.txt"
    if current.is_file():
        target = root / current.read_text().strip()
        if not target.is_dir():
            raise SnapshotError(f"current.txt aponta para snapshot inexistente: {target.name}")
        return target
    found = sorted(p for p in root.iterdir() if p.is_dir() and not p.name.startswith("."))
    if len(found) != 1:
        raise SnapshotError(f"esperado 1 snapshot sem current.txt, encontrei {len(found)}")
    return found[0]
