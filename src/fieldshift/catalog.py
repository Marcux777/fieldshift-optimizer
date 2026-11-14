# AI-assisted (OpenBot agent)
"""Catálogo agronômico, calendário e enumeração dos planos viáveis (ZARC <= 20%).

Lê ``config/catalog.toml``, ``config/soil.toml`` e a tábua ``config/zarc/*.csv`` (``load_catalog``,
``load_soil``: único I/O deste módulo). O resto é função pura sobre dados imutáveis.

Modelo (formulação "Revisão 13/11 noite" + decisões de 14/11):

* Dia-índice ``t``: 0 = 1/set do ano y; calendário de 365 dias por ano agrícola (29/fev descartado);
  horizonte de 730 dias (J1/J2 no ano y, J3/J4 no ano y+1).
* Plano ``x = (o1, o2, o3, o4)``: J1/J3 = (cultura de verão, decêndio de semeadura);
  J2/J4 = 2ª safra
  com semeadura DERIVADA (primeiro dia >= colheita anterior + margem em decêndio a 20%).
* Pousio em J2/J4 só quando nenhuma 2ª safra é viável (decisão do Development Lead); o relato de
  quantos planos usam pousio sai de ``fallow_summary``.
* ZARC: só decêndios com risco exatamente <= 20%. Não há como elevar o limite. X vazio => INVIÁVEL
  com diagnóstico (``diagnose_infeasible``); nunca se relaxa nada.
"""

from __future__ import annotations

import csv
import hashlib
import tomllib
from collections import Counter
from collections.abc import Mapping, Sequence
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

# --------------------------------------------------------------------------- calendário
DAYS_PER_YEAR = 365
N_YEARS = 2
HORIZON_DAYS = DAYS_PER_YEAR * N_YEARS
# Meses do ano agrícola (set..ago) no calendário sem 29/fev.
_MONTH_ORDER = (9, 10, 11, 12, 1, 2, 3, 4, 5, 6, 7, 8)
_MONTH_LEN = {1: 31, 2: 28, 3: 31, 4: 30, 5: 31, 6: 30, 7: 31, 8: 31, 9: 30, 10: 31, 11: 30, 12: 31}
_MONTH_START: dict[int, int] = {}
_acc = 0
for _m in _MONTH_ORDER:
    _MONTH_START[_m] = _acc
    _acc += _MONTH_LEN[_m]
assert _acc == DAYS_PER_YEAR
WINDOWS = ("J1", "J2", "J3", "J4")
ZARC_RISK_LIMIT_PCT = 20  # decisão 1 do usuário: constante; não há parâmetro para subir


def md_to_t(month: int, day: int) -> int:
    """Dia-índice (0..364) de mês/dia no ano agrícola de 365 dias (1/set = 0)."""
    if month not in _MONTH_LEN or not 1 <= day <= _MONTH_LEN[month]:
        raise ValueError(f"data inválida no calendário de 365 dias: {month}/{day}")
    return _MONTH_START[month] + day - 1


def parse_md(text: str) -> int:
    """'09-25' -> dia-índice."""
    mm, dd = text.split("-")
    return md_to_t(int(mm), int(dd))


def t_to_md(t: int) -> tuple[int, int]:
    """Dia-índice (qualquer ano do horizonte) -> (mês, dia)."""
    u = t % DAYS_PER_YEAR
    for m in reversed(_MONTH_ORDER):
        if u >= _MONTH_START[m]:
            return m, u - _MONTH_START[m] + 1
    raise AssertionError("unreachable")


def t_label(t: int) -> str:
    """'26/jan' (com sufixo ' (ano 2)' quando t >= 365)."""
    names = ("", "jan", "fev", "mar", "abr", "mai", "jun", "jul", "ago", "set", "out", "nov", "dez")
    m, d = t_to_md(t)
    return f"{d:02d}/{names[m]}" + (" (ano 2)" if t >= DAYS_PER_YEAR else "")


def dec_start_t(dec: int) -> int:
    """Primeiro dia (0..364) do decêndio ZARC ``dec`` (1..36; dec1 = 1–10/jan, dec25 = 1–10/set)."""
    if not 1 <= dec <= 36:
        raise ValueError(f"decêndio inválido: {dec}")
    month, part = (dec - 1) // 3 + 1, (dec - 1) % 3
    return md_to_t(month, 1 + 10 * part)


def dec_end_t(dec: int) -> int:
    """Último dia (inclusive) do decêndio."""
    month, part = (dec - 1) // 3 + 1, (dec - 1) % 3
    return md_to_t(month, (10, 20, _MONTH_LEN[month])[part])


def t_to_dec(t: int) -> int:
    """Decêndio ZARC (1..36) do dia-índice t (qualquer ano)."""
    m, d = t_to_md(t)
    part = 0 if d <= 10 else 1 if d <= 20 else 2
    return (m - 1) * 3 + part + 1


def dec_ordinal(dec: int) -> int:
    """Posição cronológica do decêndio no ano agrícola (dec25 -> 0, dec36 -> 11, dec1 -> 12 ...)."""
    return (dec - 25) % 36


# --------------------------------------------------------------------------- erros
class CatalogError(ValueError):
    """Configuração ou tábua ZARC inválida (falha alta; nunca há fallback silencioso)."""


# --------------------------------------------------------------------------- dados
@dataclass(frozen=True)
class Crop:
    key: str
    label: str
    species: tuple[str, ...]
    grain: bool
    windows: tuple[str, ...]
    zarc_name: str | None
    zarc_soil_key: str | None
    cycle_days: int
    phase_days: tuple[int, ...]
    brachiaria_cover_after_harvest: bool = False
    legal_window: tuple[int, int] | None = None  # (t_inicio, t_fim) dentro do ano agrícola
    is_fallow: bool = False


@dataclass(frozen=True)
class WindowSpec:
    name: str
    year_index: int
    crops: tuple[str, ...]
    sowing_rule: str  # "decendio_start" | "derived_after_previous_harvest"
    decendios: tuple[int, ...]
    fallow_allowed: str  # "false" | "true" | "only_if_no_feasible_crop"


@dataclass(frozen=True)
class Soil:
    """Solo informado: classe AD1–AD6 e textura (arenosa/media/argilosa)."""

    ad_class: str = "AD4"
    texture: str = "argilosa"
    cod_ad: Mapping[str, str] = field(default_factory=dict, compare=False, hash=False)
    cod_texture: Mapping[str, str] = field(default_factory=dict, compare=False, hash=False)

    def cod_solo(self, key: str) -> str:
        if key == "ad":
            return self.cod_ad[self.ad_class]
        if key == "texture":
            return self.cod_texture[self.texture]
        raise CatalogError(f"zarc_soil_key desconhecido: {key!r}")

    def describe(self) -> dict[str, str]:
        return {"ad_class": self.ad_class, "texture": self.texture}


@dataclass(frozen=True)
class Catalog:
    crops: tuple[Crop, ...]  # ordem do TOML; o pousio é a última entrada
    windows: Mapping[str, WindowSpec]
    emergence_days: int
    margin_days: int
    j2_search_end_t: int  # último dia (inclusive) em que a semeadura derivada pode ocorrer
    brachiaria_desiccation_days: int
    zarc: Mapping[tuple[str, str], tuple[int, ...]]  # (zarc_name, cod_solo) -> 36 riscos (%)
    default_p0: Mapping[str, Any]
    files_sha256: Mapping[str, str]
    soil_defaults: Mapping[str, str]
    cod_ad: Mapping[str, str]
    cod_texture: Mapping[str, str]

    @property
    def fallow_index(self) -> int:
        for i, c in enumerate(self.crops):
            if c.is_fallow:
                return i
        raise CatalogError("catálogo sem pousio")

    def index_of(self, key: str) -> int:
        for i, c in enumerate(self.crops):
            if c.key == key:
                return i
        raise CatalogError(f"cultura desconhecida: {key!r}")

    def zarc_row(self, crop: Crop, soil: Soil) -> tuple[int, ...]:
        if crop.zarc_name is None or crop.zarc_soil_key is None:
            raise CatalogError(f"{crop.key} não tem linha ZARC")
        key = (crop.zarc_name, soil.cod_solo(crop.zarc_soil_key))
        try:
            return self.zarc[key]
        except KeyError as exc:
            raise CatalogError(f"tábua ZARC sem linha para {key}") from exc

    def zarc_risk(self, crop: Crop, soil: Soil, dec: int) -> int:
        return self.zarc_row(crop, soil)[dec - 1]


@dataclass(frozen=True)
class Occupation:
    window: str
    crop: int  # índice no catálogo (pousio = catalog.fallow_index)
    sow_t: int | None
    emerge_t: int | None
    harvest_t: int | None  # exclusivo: ciclo vivo = [emerge_t, harvest_t)
    dec: int | None  # decêndio ZARC da semeadura

    @property
    def is_fallow(self) -> bool:
        return self.sow_t is None


@dataclass(frozen=True)
class Plan:
    """Plano imutável. ``id`` = (c1, d1, c2, c3, d3, c4): cultura/decêndio-ordinal, estável."""

    id: tuple[int, ...]
    occupations: tuple[Occupation, ...]

    def __hash__(self) -> int:
        return hash(self.id)

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Plan) and self.id == other.id

    @property
    def signature(self) -> tuple[int, int, int, int]:
        o = self.occupations
        return (o[0].crop, o[1].crop, o[2].crop, o[3].crop)

    def fallow_windows(self) -> tuple[str, ...]:
        return tuple(o.window for o in self.occupations if o.is_fallow)


@dataclass(frozen=True)
class Rejection:
    window: str
    crop: str
    dec: int | None
    reason: str
    detail: str = ""

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass(frozen=True)
class PlanSpec:
    """Plano informado pelo usuário (P0): cultura e decêndio de J1/J3 e cultura de J2/J4."""

    j1: str
    dec1: int
    j2: str
    j3: str
    dec3: int
    j4: str

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


# --------------------------------------------------------------------------- leitura (I/O)
def _sha256_file(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def load_soil(config_dir: Path, ad_class: str | None = None, texture: str | None = None) -> Soil:
    """Solo informado (padrão: ``[default]`` de soil.toml, hipótese AD4 + argilosa)."""
    cfg = tomllib.loads((Path(config_dir) / "soil.toml").read_text(encoding="utf-8"))
    cod_ad = {k: v["zarc_cod_solo"] for k, v in cfg["ad_classes"].items()}
    cod_tx = {k: v["zarc_cod_solo"] for k, v in cfg["textures"].items()}
    ad = ad_class or cfg["default"]["ad_class"]
    tx = texture or cfg["default"]["texture"]
    if ad not in cod_ad:
        raise CatalogError(f"classe AD desconhecida: {ad!r} (válidas: {sorted(cod_ad)})")
    if tx not in cod_tx:
        raise CatalogError(f"textura desconhecida: {tx!r} (válidas: {sorted(cod_tx)})")
    return Soil(ad_class=ad, texture=tx, cod_ad=cod_ad, cod_texture=cod_tx)


def _load_zarc(
    path: Path, cfg: Mapping[str, Any], names: set[str]
) -> dict[tuple[str, str], tuple[int, ...]]:
    zc = cfg["zarc"]
    with path.open(encoding=zc["csv_encoding"], newline="") as fh:
        rows = list(csv.reader(fh, delimiter=zc["csv_separator"]))
    head = rows[0]
    col = {name: i for i, name in enumerate(head)}
    dec_cols = [col[f"{zc['decendio_column_prefix']}{d}"] for d in range(1, 37)]
    out: dict[tuple[str, str], tuple[int, ...]] = {}
    for r in rows[1:]:
        if (
            r[col["geocodigo"]] != zc["filter_geocodigo"]
            or r[col["Cod_Ciclo"]] != zc["filter_cod_ciclo"]
            or r[col["Cod_Outros_Manejos"]] != zc["filter_cod_outros_manejos"]
            or r[col["Nome_cultura"]] not in names
        ):
            continue
        key = (r[col["Nome_cultura"]], r[col["Cod_Solo"]])
        if key in out:
            raise CatalogError(f"tábua ZARC com linha duplicada para {key}")
        out[key] = tuple(int(r[c]) for c in dec_cols)
    return out


def load_catalog(config_dir: Path) -> Catalog:
    """Lê catalog.toml + soil.toml + tábua ZARC (confere o SHA-256 declarado)."""
    config_dir = Path(config_dir)
    cat_path = config_dir / "catalog.toml"
    cfg = tomllib.loads(cat_path.read_text(encoding="utf-8"))
    cal = cfg["calendar"]
    if cal["days_per_agri_year"] != DAYS_PER_YEAR or cal["horizon_days"] != HORIZON_DAYS:
        raise CatalogError("calendário do catálogo diverge do módulo (365/730 dias)")
    if cfg["zarc"]["risk_limit_percent"] != ZARC_RISK_LIMIT_PCT:
        raise CatalogError("risk_limit_percent deve ser 20 (decisão 1: não relaxar)")
    zarc_path = config_dir / cfg["zarc"]["file"]
    zarc_sha = _sha256_file(zarc_path)
    if zarc_sha != cfg["zarc"]["sha256"]:
        raise CatalogError(f"SHA-256 da tábua ZARC diverge: {zarc_sha} != {cfg['zarc']['sha256']}")

    crops: list[Crop] = []
    for key, c in cfg["crops"].items():
        lw = c.get("legal_window")
        crops.append(
            Crop(
                key=key,
                label=c["label"],
                species=tuple(c["species"]),
                grain=bool(c["grain"]),
                windows=tuple(c["windows"]),
                zarc_name=c.get("zarc_name"),
                zarc_soil_key=c.get("zarc_soil_key"),
                cycle_days=int(c.get("cycle_days", 0)),
                phase_days=tuple(c.get("phase_days", ())),
                brachiaria_cover_after_harvest=bool(c.get("brachiaria_cover_after_harvest", False)),
                legal_window=(parse_md(lw[0]), parse_md(lw[1])) if lw else None,
                is_fallow=key == "pousio",
            )
        )
        if c.get("phase_days") and sum(c["phase_days"]) != c["cycle_days"]:
            raise CatalogError(f"{key}: phase_days não soma cycle_days")
    if not crops[-1].is_fallow:
        raise CatalogError("o pousio deve ser a última entrada de [crops]")

    windows: dict[str, WindowSpec] = {}
    for name, w in cfg["windows"].items():
        fa = w["fallow_allowed"]
        windows[name] = WindowSpec(
            name=name,
            year_index=int(w["agri_year_index"]),
            crops=tuple(w["crops"]),
            sowing_rule=w["sowing_rule"],
            decendios=tuple(int(d) for d in w["decendios"]),
            fallow_allowed=str(fa).lower() if isinstance(fa, bool) else fa,
        )
    if set(windows) != set(WINDOWS):
        raise CatalogError("catálogo deve definir J1..J4")

    soil_cfg = tomllib.loads((config_dir / "soil.toml").read_text(encoding="utf-8"))
    names = {c.zarc_name for c in crops if c.zarc_name}
    zarc = _load_zarc(zarc_path, cfg, names)
    succ = cfg["succession"]
    files = {
        "catalog.toml": _sha256_file(cat_path),
        "soil.toml": _sha256_file(config_dir / "soil.toml"),
        "zarc": zarc_sha,
    }
    return Catalog(
        crops=tuple(crops),
        windows=windows,
        emergence_days=int(cal["emergence_days"]),
        margin_days=int(cal["operational_margin_days"]),
        j2_search_end_t=md_to_t(4, 30),
        brachiaria_desiccation_days=int(succ["brachiaria_desiccation_days_before_next_sowing"]),
        zarc=zarc,
        default_p0=cfg["current_plan_default"],
        files_sha256=files,
        soil_defaults={k: soil_cfg["default"][k] for k in ("ad_class", "texture")},
        cod_ad={k: v["zarc_cod_solo"] for k, v in soil_cfg["ad_classes"].items()},
        cod_texture={k: v["zarc_cod_solo"] for k, v in soil_cfg["textures"].items()},
    )


def default_p0_spec(catalog: Catalog) -> PlanSpec:
    p = catalog.default_p0
    return PlanSpec(
        j1=p["J1"]["crop"],
        dec1=int(p["J1"]["decendio"]),
        j2=p["J2"]["crop"],
        j3=p["J3"]["crop"],
        dec3=int(p["J3"]["decendio"]),
        j4=p["J4"]["crop"],
    )


# --------------------------------------------------------------------------- enumeração
def _risk_limit(zarc_risk: float) -> int:
    pct = round(zarc_risk * 100)
    if pct > ZARC_RISK_LIMIT_PCT:
        raise CatalogError("o limite ZARC não pode passar de 20% (decisão 1 do usuário)")
    return pct


def _occupation(catalog: Catalog, window: str, crop_idx: int, sow_t: int) -> Occupation:
    crop = catalog.crops[crop_idx]
    e = sow_t + catalog.emergence_days
    return Occupation(window, crop_idx, sow_t, e, e + crop.cycle_days, t_to_dec(sow_t))


def _fallow(catalog: Catalog, window: str) -> Occupation:
    return Occupation(window, catalog.fallow_index, None, None, None, None)


def _allowed(risk: int, limit: int) -> bool:
    return 0 < risk <= limit


def _first_options(
    catalog: Catalog, soil: Soil, limit: int, wname: str, rejections: list[Rejection]
) -> list[Occupation]:
    """Opções de J1 (sem deslocar o ano): semeadura no 1º dia do decêndio permitido."""
    win = catalog.windows[wname]
    out: list[Occupation] = []
    for key in win.crops:
        idx = catalog.index_of(key)
        crop = catalog.crops[idx]
        for dec in win.decendios:
            risk = catalog.zarc_risk(crop, soil, dec)
            if risk == 0:
                rejections.append(
                    Rejection(wname, key, dec, "ZARC_NAO_INDICADO", "risco 0 = não recomendado")
                )
                continue
            if risk > limit:
                rejections.append(
                    Rejection(wname, key, dec, "ZARC_RISCO_ACIMA", f"risco {risk}% > {limit}%")
                )
                continue
            s = dec_start_t(dec)
            if crop.legal_window is not None:
                lo, hi = crop.legal_window
                if not (dec_start_t(dec) >= lo and dec_end_t(dec) <= hi):
                    rejections.append(
                        Rejection(
                            wname,
                            key,
                            dec,
                            "JANELA_LEGAL",
                            "decêndio fora da janela legal de semeadura",
                        )
                    )
                    continue
            occ = _occupation(catalog, wname, idx, s)
            if occ.harvest_t > DAYS_PER_YEAR:
                rejections.append(
                    Rejection(wname, key, dec, "FORA_DO_ANO", "colheita depois de 31/ago")
                )
                continue
            out.append(occ)
    return out


def _derived_second(
    catalog: Catalog,
    soil: Soil,
    limit: int,
    wname: str,
    prev_harvest: int,
    crop_key: str,
    rejections: list[Rejection],
) -> Occupation | None:
    """2ª safra: primeiro dia t >= colheita + margem em decêndio permitido; None se não houver."""
    idx = catalog.index_of(crop_key)
    crop = catalog.crops[idx]
    t0 = prev_harvest + catalog.margin_days
    for t in range(t0, catalog.j2_search_end_t + 1):
        if _allowed(catalog.zarc_risk(crop, soil, t_to_dec(t)), limit):
            occ = _occupation(catalog, wname, idx, t)
            if occ.harvest_t > DAYS_PER_YEAR:
                rejections.append(
                    Rejection(wname, crop_key, occ.dec, "FORA_DO_ANO", "colheita depois de 31/ago")
                )
                return None
            return occ
    rejections.append(
        Rejection(
            wname,
            crop_key,
            t_to_dec(min(t0, DAYS_PER_YEAR - 1)),
            "SEM_DECENDIO_ZARC_APOS_COLHEITA",
            f"colheita anterior em {t_label(prev_harvest)}: "
            f"nenhum decêndio a <= {limit}% até 30/abr",
        )
    )
    return None


def _year_blocks(
    catalog: Catalog, soil: Soil, limit: int, rejections: list[Rejection]
) -> list[tuple[Occupation, Occupation]]:
    blocks: list[tuple[Occupation, Occupation]] = []
    w2 = catalog.windows["J2"]
    for o1 in _first_options(catalog, soil, limit, "J1", rejections):
        viable: list[Occupation] = []
        for key in w2.crops:
            if catalog.crops[catalog.index_of(key)].is_fallow:
                continue
            occ = _derived_second(catalog, soil, limit, "J2", o1.harvest_t, key, rejections)
            if occ is not None:
                viable.append(occ)
        mode = w2.fallow_allowed
        if viable:
            blocks.extend((o1, o2) for o2 in viable)
            if mode == "true":
                blocks.append((o1, _fallow(catalog, "J2")))
        elif mode in ("only_if_no_feasible_crop", "true"):
            blocks.append((o1, _fallow(catalog, "J2")))
        else:
            rejections.append(
                Rejection("J2", "-", o1.dec, "SEM_SEGUNDA_SAFRA", "pousio não permitido")
            )
    return blocks


def _shift(occ: Occupation, window: str, offset: int) -> Occupation:
    if occ.is_fallow:
        return Occupation(window, occ.crop, None, None, None, None)
    assert occ.sow_t is not None and occ.emerge_t is not None and occ.harvest_t is not None
    return Occupation(
        window, occ.crop, occ.sow_t + offset, occ.emerge_t + offset, occ.harvest_t + offset, occ.dec
    )


def _plan_id(o: Sequence[Occupation]) -> tuple[int, ...]:
    def ordn(x: Occupation) -> int:
        return dec_ordinal(x.dec) if x.dec is not None else -1

    return (o[0].crop, ordn(o[0]), o[1].crop, o[2].crop, ordn(o[2]), o[3].crop)


def _last_end(b: tuple[Occupation, Occupation]) -> int:
    """Instante a partir do qual o talhão está livre (fim da última ocupação do ano)."""
    ends = [x.harvest_t for x in b if x.harvest_t is not None]
    return max(ends)


def enumerate_plans(
    catalog: Catalog, soil: Soil, zarc_risk: float = 0.20
) -> tuple[list[Plan], list[Rejection]]:
    """Todos os planos viáveis (ordenados por ID) e a lista de rejeições com motivo.

    ``zarc_risk`` só aceita <= 0.20 (a decisão do usuário proíbe relaxar). Lista vazia => INVIÁVEL.
    """
    limit = _risk_limit(zarc_risk)
    rejections: list[Rejection] = []
    blocks = _year_blocks(catalog, soil, limit, rejections)
    plans: list[Plan] = []
    for b1 in blocks:
        for b2 in blocks:
            occ3 = _shift(b2[0], "J3", DAYS_PER_YEAR)
            occ4 = _shift(b2[1], "J4", DAYS_PER_YEAR)
            # S2: sem sobreposição entre anos (h(J2) <= s(J3)) e tudo dentro do horizonte.
            if _last_end(b1) > occ3.sow_t:  # type: ignore[operator]
                rejections.append(
                    Rejection("J3", catalog.crops[occ3.crop].key, occ3.dec, "SOBREPOSICAO_J2_J3")
                )
                continue
            if occ4.harvest_t is not None and occ4.harvest_t > HORIZON_DAYS:
                rejections.append(
                    Rejection("J4", catalog.crops[occ4.crop].key, occ4.dec, "FORA_DO_HORIZONTE")
                )
                continue
            occs = (b1[0], b1[1], occ3, occ4)
            plans.append(Plan(_plan_id(occs), occs))
    plans.sort(key=lambda p: p.id)
    ids = [p.id for p in plans]
    if len(set(ids)) != len(ids):
        raise CatalogError("IDs de plano duplicados")
    return plans, rejections


def index_plans(plans: Sequence[Plan]) -> dict[tuple[int, ...], Plan]:
    return {p.id: p for p in plans}


# ------------------------------------------------------------------- verificador independente
def verify_plan(catalog: Catalog, soil: Soil, plan: Plan, zarc_risk: float = 0.20) -> list[str]:
    """Violações do plano (lista vazia = aprovado). Reimplementa as restrições a partir das datas.

    Não confere se a semeadura de J2/J4 é a "primeira possível" (regra de derivação), só as
    restrições: ZARC <= 20% no decêndio, janela legal, sem sobreposição, tudo dentro do
    ano/horizonte.
    """
    limit = _risk_limit(zarc_risk)
    errs: list[str] = []
    occs = plan.occupations
    if tuple(o.window for o in occs) != WINDOWS:
        return ["janelas fora de ordem"]
    for o in occs:
        crop = catalog.crops[o.crop]
        if crop.is_fallow:
            if catalog.windows[o.window].fallow_allowed == "false":
                errs.append(f"{o.window}: pousio não permitido na janela")
            continue
        if o.window not in catalog.windows or crop.key not in catalog.windows[o.window].crops:
            errs.append(f"{o.window}: cultura {crop.key} não pertence à janela")
            continue
        if o.is_fallow:
            continue
        assert o.sow_t is not None and o.harvest_t is not None and o.emerge_t is not None
        if (
            o.emerge_t != o.sow_t + catalog.emergence_days
            or o.harvest_t != o.emerge_t + crop.cycle_days
        ):
            errs.append(f"{o.window}: datas inconsistentes com o ciclo")
        dec = t_to_dec(o.sow_t)
        risk = catalog.zarc_risk(crop, soil, dec)
        if not _allowed(risk, limit):
            errs.append(f"{o.window}: ZARC {risk}% no decêndio {dec} (limite {limit}%)")
        if crop.legal_window is not None:
            u = o.sow_t % DAYS_PER_YEAR
            lo, hi = crop.legal_window
            if not (dec_start_t(dec) >= lo and dec_end_t(dec) <= hi and lo <= u):
                errs.append(f"{o.window}: fora da janela legal")
        yr0 = DAYS_PER_YEAR * catalog.windows[o.window].year_index
        if not (yr0 <= o.sow_t and o.harvest_t <= yr0 + DAYS_PER_YEAR):
            errs.append(f"{o.window}: ocupação fora do próprio ano agrícola")
    j1, j2, j3, j4 = occs
    if not j1.is_fallow and j1.sow_t is not None and j1.harvest_t is not None:
        if not j2.is_fallow and j2.sow_t < j1.harvest_t + catalog.margin_days:  # type: ignore[operator]
            errs.append("J2 começa antes da colheita de J1 (sobreposição)")
        if not j3.is_fallow and j3.sow_t < j1.harvest_t:  # type: ignore[operator]
            errs.append("J3 começa antes da colheita de J1")
    for prev, nxt, label in ((j2, j3, "J2/J3"), (j3, j4, "J3/J4")):
        if prev.harvest_t is not None and nxt.sow_t is not None:
            margin = catalog.margin_days if label == "J3/J4" else 0
            if nxt.sow_t < prev.harvest_t + margin:
                errs.append(f"sobreposição {label}")
    if j1.is_fallow or j3.is_fallow:
        errs.append("J1/J3 devem ter cultura de grão")
    return errs


# --------------------------------------------------------------------------- P0 e relatos
def plan_id_from_spec(catalog: Catalog, spec: PlanSpec) -> tuple[int, ...]:
    return (
        catalog.index_of(spec.j1),
        dec_ordinal(spec.dec1),
        catalog.index_of(spec.j2),
        catalog.index_of(spec.j3),
        dec_ordinal(spec.dec3),
        catalog.index_of(spec.j4),
    )


def resolve_spec(
    catalog: Catalog, soil: Soil, spec: PlanSpec, plans: Sequence[Plan], zarc_risk: float = 0.20
) -> tuple[Plan | None, list[str]]:
    """Plano do usuário dentro de X, ou ``None`` com os motivos ("inviável sob ZARC <= 20%")."""
    try:
        wanted = plan_id_from_spec(catalog, spec)
    except (CatalogError, ValueError) as exc:
        return None, [f"plano inválido: {exc}"]
    found = index_plans(plans).get(wanted)
    if found is not None:
        return found, []
    limit = _risk_limit(zarc_risk)
    reasons: list[str] = []
    for w, key, dec in (("J1", spec.j1, spec.dec1), ("J3", spec.j3, spec.dec3)):
        idx = catalog.index_of(key)
        crop = catalog.crops[idx]
        if w not in {x for x in crop.windows}:
            reasons.append(f"{w}: {key} não pertence à janela")
            continue
        rej: list[Rejection] = []
        opts = _first_options(catalog, soil, limit, "J1", rej)
        ok = any(o.crop == idx and o.dec == dec for o in opts)
        if not ok:
            hit = [r for r in rej if r.crop == key and r.dec == dec]
            reasons.append(
                f"{w}: {key} no decêndio {dec} não é permitido ({hit[0].reason}: {hit[0].detail})"
                if hit
                else f"{w}: {key} no decêndio {dec} fora dos decêndios candidatos"
            )
    if reasons:
        return None, reasons
    # J1/J3 ok: o problema está em J2/J4 (cultura inviável, ou pousio quando há 2ª safra viável).
    for w, k1, d1, k2 in (("J2", spec.j1, spec.dec1, spec.j2), ("J4", spec.j3, spec.dec3, spec.j4)):
        rej = []
        o1 = next(
            o
            for o in _first_options(catalog, soil, limit, "J1", [])
            if o.crop == catalog.index_of(k1) and o.dec == d1
        )
        crop2 = catalog.crops[catalog.index_of(k2)]
        if crop2.is_fallow:
            viable = [
                k
                for k in catalog.windows["J2"].crops
                if not catalog.crops[catalog.index_of(k)].is_fallow
                and _derived_second(catalog, soil, limit, "J2", o1.harvest_t, k, rej) is not None
            ]
            if viable:
                reasons.append(
                    f"{w}: pousio não permitido: há 2ª safra viável ({', '.join(viable)})"
                )
        else:
            if _derived_second(catalog, soil, limit, "J2", o1.harvest_t, k2, rej) is None:
                reasons.append(
                    f"{w}: {k2} inviável sob ZARC <= {limit}% depois da colheita de {k1} "
                    f"({rej[-1].detail})"
                )
    return None, reasons or ["plano fora de X (sobreposição entre anos ou fora do horizonte)"]


def describe_plan(catalog: Catalog, plan: Plan) -> dict[str, Any]:
    """Resumo legível e serializável do plano."""
    occ = []
    for o in plan.occupations:
        crop = catalog.crops[o.crop]
        occ.append(
            {
                "window": o.window,
                "crop": crop.key,
                "label": crop.label,
                "sow": None if o.sow_t is None else t_label(o.sow_t),
                "sow_t": o.sow_t,
                "harvest_t": o.harvest_t,
                "zarc_dec": o.dec,
            }
        )
    return {"id": list(plan.id), "occupations": occ}


def fallow_summary(plans: Sequence[Plan]) -> dict[str, int]:
    """Quantos planos de X usam pousio (J2, J4, ambos) — relato exigido pelo Development Lead."""
    c = Counter()
    for p in plans:
        fw = p.fallow_windows()
        c["total"] += 1
        c["com_pousio"] += bool(fw)
        c["pousio_J2"] += "J2" in fw
        c["pousio_J4"] += "J4" in fw
        c["pousio_J2_e_J4"] += len(fw) == 2
    return dict(c)


def summarize_rejections(rejections: Sequence[Rejection]) -> dict[str, int]:
    c = Counter(f"{r.window}:{r.crop}:{r.reason}" for r in rejections)
    return dict(sorted(c.items()))


def diagnose_infeasible(rejections: Sequence[Rejection]) -> dict[str, Any]:
    """Diagnóstico de X vazio (nada é relaxado): rejeições por cultura, janela e motivo."""
    return {
        "status": "INVIAVEL",
        "message": (
            "Nenhum plano viável com ZARC <= 20% para o solo informado. O limite, o grupo de ciclo "
            "e o solo não foram alterados."
        ),
        "rejections_by_reason": summarize_rejections(rejections),
        "n_rejections": len(rejections),
    }
