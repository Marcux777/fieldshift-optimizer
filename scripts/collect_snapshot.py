# AI-assisted (OpenBot agent)
"""Coletor idempotente do snapshot NASA POWER (único código do projeto que usa rede).

Uso (no contêiner)::

    docker compose exec -T dev python scripts/collect_snapshot.py

Uma requisição por célula (o POWER pede isso). Se já existir snapshot com a mesma requisição,
nada é baixado: o existente é só verificado. ``--refresh`` força nova coleta; se o conteúdo
for idêntico, o snapshot é o mesmo (mesmo id); se a NASA revisou os dados, nasce um NOVO
snapshot e ``current.txt`` só muda com ``--set-current``.
"""

from __future__ import annotations

import argparse
import json
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from fieldshift import data  # noqa: E402

API = "https://power.larc.nasa.gov/api/temporal/daily/point"


def build_request(lat: float, lon: float, start: str, end: str, community: str) -> tuple[str, dict]:
    params = {
        "parameters": ",".join(data.COLUMNS),
        "community": community,
        "longitude": f"{lon:g}",
        "latitude": f"{lat:g}",
        "start": start,
        "end": end,
        "format": "JSON",
    }
    return f"{API}?{urllib.parse.urlencode(params, safe=',')}", params


def find_existing(root: Path, url: str) -> Path | None:
    if not root.is_dir():
        return None
    for d in sorted(p for p in root.iterdir() if p.is_dir() and not p.name.startswith(".")):
        try:
            if data.read_manifest(d)["request"]["url"] == url:
                return d
        except (data.SnapshotError, KeyError):
            continue
    return None


def fetch(url: str, attempts: int = 2) -> bytes:
    last: Exception | None = None
    for i in range(attempts):
        try:
            req = urllib.request.Request(url, headers={"User-Agent": "fieldshift-collector/0.1"})
            with urllib.request.urlopen(req, timeout=120) as resp:
                if resp.status != 200:
                    raise RuntimeError(f"HTTP {resp.status}")
                return resp.read()
        except (urllib.error.URLError, RuntimeError, TimeoutError) as exc:
            last = exc
            if i + 1 < attempts:
                time.sleep(10)
    raise SystemExit(f"falha ao coletar do POWER: {last!r}")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawTextHelpFormatter)
    ap.add_argument("--root", type=Path, default=ROOT / "data" / "snapshots")
    ap.add_argument("--lat", type=float, default=-18.0)
    ap.add_argument("--lon", type=float, default=-50.625)
    ap.add_argument("--start", default="19810101")
    ap.add_argument("--end", default="20251231")
    ap.add_argument("--community", default="AG")
    ap.add_argument("--refresh", action="store_true", help="ignora snapshot existente e baixa")
    ap.add_argument(
        "--set-current", action="store_true", help="aponta current.txt para o resultado"
    )
    args = ap.parse_args(argv)

    url, params = build_request(args.lat, args.lon, args.start, args.end, args.community)
    args.root.mkdir(parents=True, exist_ok=True)

    existing = None if args.refresh else find_existing(args.root, url)
    if existing is not None:
        data.verify_manifest(existing)
        snap = existing
        print(f"snapshot existente verificado (sem rede): {snap.name}")
    else:
        print(f"coletando (1 requisição): {url}")
        raw = fetch(url)
        snap = data.write_snapshot(raw, args.root, request_url=url, request_params=params)
        print(f"snapshot gravado/verificado: {snap.name}")

    current = args.root / "current.txt"
    if args.set_current or not current.exists():
        current.write_text(snap.name + "\n")
        print(f"current.txt -> {snap.name}")
    manifest = data.verify_manifest(snap)
    print(
        json.dumps(
            {
                "snapshot_id": manifest["snapshot_id"],
                "rows": manifest["rows"],
                "date_min": manifest["date_min"],
                "date_max": manifest["date_max"],
            }
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
