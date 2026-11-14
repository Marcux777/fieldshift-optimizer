# AI-assisted (OpenBot agent)
"""Testes do catálogo: calendário, enumeração ZARC <= 20%, IDs, verificador, INVIÁVEL, P0.

A tabela de verificação manual vem de ``config/zarc/VERIFICATION.md`` (Methods Scientist); a leitura
da tábua ZARC aqui é independente da usada por ``catalog.py``.
"""

from __future__ import annotations

import csv
import random
import shutil
from dataclasses import replace
from pathlib import Path

import pytest

from fieldshift import catalog as C

ROOT = Path(__file__).resolve().parents[1]
CONFIG = ROOT / "config"
ZARC_CSV = CONFIG / "zarc" / "zarc-tabua-rio-verde-5218805-2026-2027.csv"


@pytest.fixture(scope="module")
def cat() -> C.Catalog:
    return C.load_catalog(CONFIG)


@pytest.fixture(scope="module")
def soil() -> C.Soil:
    return C.load_soil(CONFIG)


@pytest.fixture(scope="module")
def xset(cat, soil):
    plans, rej = C.enumerate_plans(cat, soil)
    return plans, rej


def raw_zarc(name: str, cod_solo: str) -> list[int]:
    """Leitura independente da tábua: Grupo I (Cod_Ciclo 20), sequeiro (Cod_Outros_Manejos 1)."""
    with ZARC_CSV.open(encoding="utf-8-sig", newline="") as fh:
        rows = list(csv.DictReader(fh, delimiter=";"))
    hit = [
        r
        for r in rows
        if r["Nome_cultura"] == name
        and r["Cod_Ciclo"] == "20"
        and r["Cod_Outros_Manejos"] == "1"
        and r["Cod_Solo"] == cod_solo
        and r["geocodigo"] == "5218805"
    ]
    assert len(hit) == 1, (name, cod_solo, len(hit))
    return [int(hit[0][f"dec{d}"]) for d in range(1, 37)]


# ------------------------------------------------------------------ calendário
def test_calendar_day_index():
    assert C.md_to_t(9, 1) == 0
    assert C.md_to_t(10, 1) == 30
    assert C.md_to_t(1, 1) == 122
    assert C.md_to_t(8, 31) == 364
    with pytest.raises(ValueError):
        C.md_to_t(2, 29)  # 29/fev não existe no calendário de 365 dias
    assert C.t_to_md(40) == (10, 11)
    assert C.t_to_md(40 + 365) == (10, 11)
    assert C.t_label(147) == "26/jan"


def test_decendios():
    assert (C.dec_start_t(25), C.dec_end_t(25)) == (0, 9)  # 1–10/set
    assert C.dec_start_t(29) == 40  # 11/out
    assert C.dec_end_t(6) - C.dec_start_t(6) + 1 == 8  # 21–28/fev
    assert C.dec_end_t(3) - C.dec_start_t(3) + 1 == 11  # 21–31/jan
    assert C.t_to_dec(147) == 3
    assert [C.dec_ordinal(d) for d in (25, 36, 1, 24)] == [0, 11, 12, 35]
    # decêndios cobrem o ano sem lacuna nem sobreposição
    covered = []
    for d in range(1, 37):
        covered += list(range(C.dec_start_t(d), C.dec_end_t(d) + 1))
    assert sorted(covered) == list(range(365))


# ----------------------------------------------------------- tabela de verificação do Methods
THREE = {"milho_2a": 0, "milho_2a_braquiaria": 5, "sorgo_2a": 0}  # colheita extra (115 vs 110 d)
# J1 -> (colheita J1, {2ª safra: (semeadura, colheita)}), AD4 + argilosa (VERIFICATION.md §2)
EXPECTED_J1_J2 = {
    ("soja", 28): (
        137,
        {"milho_2a": (137, 254), "milho_2a_braquiaria": (137, 259), "sorgo_2a": (137, 254)},
    ),
    ("soja", 29): (
        147,
        {"milho_2a": (147, 264), "milho_2a_braquiaria": (147, 269), "sorgo_2a": (147, 264)},
    ),
    ("soja", 30): (
        157,
        {"milho_2a": (157, 274), "milho_2a_braquiaria": (157, 279), "sorgo_2a": (157, 274)},
    ),
    ("soja", 31): (168, {"sorgo_2a": (168, 285)}),
    ("soja", 32): (178, {"sorgo_2a": (178, 295)}),
    ("soja", 33): (188, {"sorgo_2a": (188, 305)}),
    ("soja", 34): (198, {"sorgo_2a": (198, 315)}),
    ("soja", 35): (208, {}),
    ("soja", 36): (218, {}),
    ("milho_1a", 30): (
        157,
        {"milho_2a": (157, 274), "milho_2a_braquiaria": (157, 279), "sorgo_2a": (157, 274)},
    ),
    ("milho_1a", 31): (168, {"sorgo_2a": (168, 285)}),
    ("milho_1a", 32): (178, {"sorgo_2a": (178, 295)}),
    ("milho_1a", 33): (188, {"sorgo_2a": (188, 305)}),
    ("milho_1a", 34): (198, {"sorgo_2a": (198, 315)}),
    ("milho_1a", 35): (208, {}),
    ("milho_1a", 36): (218, {}),
}


def test_verification_table_methods(cat, xset):
    plans, _ = xset
    got: dict[tuple[str, int], tuple[int, dict]] = {}
    for p in plans:
        o1, o2 = p.occupations[0], p.occupations[1]
        key = (cat.crops[o1.crop].key, o1.dec)
        h1 = o1.harvest_t
        opts = got.setdefault(key, (h1, {}))[1]
        if not o2.is_fallow:
            opts[cat.crops[o2.crop].key] = (o2.sow_t, o2.harvest_t)
    assert set(got) == set(EXPECTED_J1_J2)
    for key, (h1, opts) in EXPECTED_J1_J2.items():
        assert got[key] == (h1, opts), key


def test_viable_options_per_j1_only_pousio_when_no_second_crop(cat, xset):
    plans, _ = xset
    for p in plans:
        o1, o2 = p.occupations[0], p.occupations[1]
        if o2.is_fallow:
            assert not EXPECTED_J1_J2[(cat.crops[o1.crop].key, o1.dec)][1]  # só se nada é viável


# ------------------------------------------------------------------ |X| e ZARC 20%
@pytest.mark.parametrize(
    ("ad", "n"),
    [("AD1", 169), ("AD2", 289), ("AD3", 400), ("AD4", 576), ("AD5", 729), ("AD6", 961)],
)
def test_size_of_x_matches_methods_table(cat, ad, n):
    plans, _ = C.enumerate_plans(cat, C.load_soil(CONFIG, ad_class=ad))
    assert len(plans) == n


def test_fallow_report_default_soil(xset):
    plans, _ = xset
    f = C.fallow_summary(plans)
    # 24 módulos por ano, 4 só com pousio forçado: 24² - 20² = 176 planos com pousio
    assert f == {
        "total": 576,
        "com_pousio": 176,
        "pousio_J2": 96,
        "pousio_J4": 96,
        "pousio_J2_e_J4": 16,
    }


def test_every_sowing_is_exactly_20_percent_per_independent_reading(cat, soil, xset):
    plans, _ = xset
    for p in plans:
        for o in p.occupations:
            if o.is_fallow:
                continue
            crop = cat.crops[o.crop]
            cod = soil.cod_solo(crop.zarc_soil_key)
            risk = raw_zarc(crop.zarc_name, cod)[C.t_to_dec(o.sow_t) - 1]
            assert risk == 20, (p.id, o)


def test_ids_unique_sorted_and_rederivable(cat, xset):
    plans, _ = xset
    ids = [p.id for p in plans]
    assert ids == sorted(set(ids))
    for p in plans[:: max(1, len(plans) // 40)]:
        o = p.occupations

        def ordn(x):
            return C.dec_ordinal(x.dec) if x.dec is not None else -1

        assert p.id == (o[0].crop, ordn(o[0]), o[1].crop, o[2].crop, ordn(o[2]), o[3].crop)


def test_ids_do_not_depend_on_enumeration_order(cat, soil, xset):
    plans, _ = xset
    w = dict(cat.windows)
    w["J2"] = replace(w["J2"], crops=tuple(reversed(w["J2"].crops)))
    w["J1"] = replace(w["J1"], crops=tuple(reversed(w["J1"].crops)))
    plans2, _ = C.enumerate_plans(replace(cat, windows=w), soil)
    assert {p.id: p for p in plans2} == {p.id: p for p in plans}
    assert [p.id for p in plans2] == [p.id for p in plans]  # sempre ordenado por ID


def test_limit_cannot_be_raised(cat, soil):
    with pytest.raises(C.CatalogError):
        C.enumerate_plans(cat, soil, zarc_risk=0.30)


def test_stricter_limit_gives_inviavel_not_relaxation(cat, soil):
    plans, rej = C.enumerate_plans(cat, soil, zarc_risk=0.10)  # nenhuma linha tem risco 10%
    assert plans == []
    diag = C.diagnose_infeasible(rej)
    assert diag["status"] == "INVIAVEL"
    assert diag["n_rejections"] > 0
    assert any("ZARC_RISCO_ACIMA" in k for k in diag["rejections_by_reason"])


def test_empty_x_is_inviavel_with_diagnosis(cat, soil):
    zarc = {k: tuple(30 if v == 20 else v for v in row) for k, row in cat.zarc.items()}
    plans, rej = C.enumerate_plans(replace(cat, zarc=zarc), soil)
    assert plans == []
    diag = C.diagnose_infeasible(rej)
    assert diag["status"] == "INVIAVEL"
    assert "não foram alterados" in diag["message"]
    assert any(r.reason == "ZARC_RISCO_ACIMA" and r.crop == "soja" for r in rej)


def test_rejections_carry_reasons(xset):
    _, rej = xset
    by = {(r.window, r.crop, r.dec): r for r in rej}
    assert by[("J1", "soja", 27)].reason == "ZARC_RISCO_ACIMA"  # 21–30/set a 40%
    assert "40" in by[("J1", "soja", 27)].detail
    assert by[("J1", "milho_1a", 29)].reason == "ZARC_RISCO_ACIMA"  # AD4: 30%
    assert by[("J1", "soja", 25)].reason == "ZARC_NAO_INDICADO"
    codes = {r.reason for r in rej}
    assert "SEM_DECENDIO_ZARC_APOS_COLHEITA" in codes
    assert C.summarize_rejections(rej)["J1:soja:ZARC_RISCO_ACIMA"] == 1


def test_legal_window_excludes_dec1_for_soja(cat, soil):
    w = dict(cat.windows)
    w["J1"] = replace(w["J1"], decendios=(1, *w["J1"].decendios))
    plans, rej = C.enumerate_plans(replace(cat, windows=w), soil)
    assert len(plans) == 576  # dec1 (1–10/jan, 20% no ZARC) continua fora: janela legal até 02/jan
    hit = [r for r in rej if r.crop == "soja" and r.dec == 1]
    assert hit and hit[0].reason == "JANELA_LEGAL"


# ------------------------------------------------------------------ verificador independente
def test_all_enumerated_plans_pass_verifier(cat, soil, xset):
    plans, _ = xset
    for p in plans:
        assert C.verify_plan(cat, soil, p) == [], p.id


def _modify(plan: C.Plan, idx: int, **kw) -> C.Plan:
    occs = list(plan.occupations)
    occs[idx] = replace(occs[idx], **kw)
    return C.Plan(plan.id, tuple(occs))


def _find(cat, plans, crops, decs=None):
    for p in plans:
        keys = tuple(cat.crops[o.crop].key for o in p.occupations)
        if keys == crops and (decs is None or (p.occupations[0].dec, p.occupations[2].dec) == decs):
            return p
    raise AssertionError(f"plano {crops} não encontrado")


def test_verifier_rejects_30_percent_decendio(cat, soil, xset):
    plans, _ = xset
    p = _find(cat, plans, ("soja", "milho_2a", "soja", "milho_2a"), (29, 29))
    bad = _modify(
        p,
        0,
        sow_t=C.dec_start_t(27),
        emerge_t=C.dec_start_t(27) + 7,
        harvest_t=C.dec_start_t(27) + 107,
        dec=27,
    )  # dec27 = 40%
    errs = C.verify_plan(cat, soil, bad)
    assert any("ZARC 40%" in e for e in errs)


def test_verifier_rejects_overlap_j1_j2(cat, soil, xset):
    plans, _ = xset
    p = _find(cat, plans, ("soja", "milho_2a", "soja", "milho_2a"), (29, 29))
    o2 = p.occupations[1]
    early = o2.sow_t - 5  # J2 semeia antes da colheita de J1
    bad = _modify(
        p, 1, sow_t=early, emerge_t=early + 7, harvest_t=early + 117, dec=C.t_to_dec(early)
    )
    assert any("sobreposição" in e for e in C.verify_plan(cat, soil, bad))


def test_verifier_rejects_overlap_j2_j3_and_wrong_window(cat, soil, xset):
    plans, _ = xset
    p = _find(cat, plans, ("soja", "milho_2a", "soja", "milho_2a"), (29, 29))
    j3 = p.occupations[2]
    bad = _modify(p, 2, sow_t=250 + 365 - 365, emerge_t=257, harvest_t=357, dec=C.t_to_dec(250))
    assert any("J2/J3" in e or "J3" in e for e in C.verify_plan(cat, soil, bad))
    soy = cat.index_of("soja")
    bad2 = _modify(p, 1, crop=soy)  # soja em J2 não pertence à janela
    assert any("não pertence" in e for e in C.verify_plan(cat, soil, bad2))
    assert j3.window == "J3"


def test_verifier_rejects_late_second_crop(cat, soil, xset):
    plans, _ = xset
    p = _find(cat, plans, ("soja", "milho_2a", "soja", "milho_2a"), (29, 29))
    late = C.dec_start_t(5)  # 11/fev: milho 2ª AD4 só vai até dec4 (a 40%)
    bad = _modify(p, 1, sow_t=late, emerge_t=late + 7, harvest_t=late + 117, dec=5)
    assert any("ZARC" in e for e in C.verify_plan(cat, soil, bad))


# ------------------------------------------------------------------ P0 (plano atual)
def test_default_p0_is_viable_ad4(cat, soil, xset):
    plans, _ = xset
    spec = C.default_p0_spec(cat)
    assert spec == C.PlanSpec("soja", 29, "milho_2a", "soja", 29, "milho_2a")
    plan, why = C.resolve_spec(cat, soil, spec, plans)
    assert why == [] and plan is not None
    d = C.describe_plan(cat, plan)
    assert [(o["sow"], o["harvest_t"]) for o in d["occupations"][:2]] == [
        ("11/out", 147),
        ("26/jan", 264),
    ]


def test_default_p0_inviavel_in_ad1(cat):
    soil1 = C.load_soil(CONFIG, ad_class="AD1")
    plans, _ = C.enumerate_plans(cat, soil1)
    plan, why = C.resolve_spec(cat, soil1, C.default_p0_spec(cat), plans)
    assert plan is None
    assert any("soja" in r and "ZARC" in r for r in why)  # soja dec29 a 30% em AD1


def test_p0_with_fallow_when_second_crop_viable_is_outside_x(cat, soil, xset):
    plans, _ = xset
    spec = C.PlanSpec("soja", 29, "pousio", "soja", 29, "milho_2a")
    plan, why = C.resolve_spec(cat, soil, spec, plans)
    assert plan is None and any("pousio não permitido" in r for r in why)


def test_p0_second_crop_after_window_is_outside_x(cat, soil, xset):
    plans, _ = xset
    spec = C.PlanSpec(
        "soja", 31, "milho_2a", "soja", 29, "milho_2a"
    )  # soja 1/nov: milho 2ª inviável
    plan, why = C.resolve_spec(cat, soil, spec, plans)
    assert plan is None and any("milho_2a inviável" in r for r in why)


# ------------------------------------------------------------------ I/O e falhas explícitas
def _copy_config(tmp_path: Path) -> Path:
    dst = tmp_path / "config"
    shutil.copytree(CONFIG, dst)
    return dst


def test_zarc_hash_mismatch_fails_loudly(tmp_path):
    cfg = _copy_config(tmp_path)
    csv_path = cfg / "zarc" / ZARC_CSV.name
    csv_path.write_bytes(csv_path.read_bytes() + b"\n")
    with pytest.raises(C.CatalogError, match="SHA-256"):
        C.load_catalog(cfg)


def test_duplicate_zarc_row_fails_loudly(tmp_path):
    cfg = _copy_config(tmp_path)
    csv_path = cfg / "zarc" / ZARC_CSV.name
    lines = csv_path.read_bytes().split(b"\n")
    soja = next(ln for ln in lines[1:] if ln.startswith(b"Soja;") and b";20;14;" in ln)
    csv_path.write_bytes(csv_path.read_bytes().rstrip(b"\n") + b"\n" + soja + b"\n")
    import hashlib

    new = hashlib.sha256(csv_path.read_bytes()).hexdigest()
    cat_toml = cfg / "catalog.toml"
    cat_toml.write_text(
        cat_toml.read_text().replace(
            "88f24986b6063e0eaba7bd0cd83f97d9a623f5c5f9ada156711e318b9e3218ad", new
        )
    )
    with pytest.raises(C.CatalogError, match="duplicada"):
        C.load_catalog(cfg)


def test_unknown_soil_values_fail():
    with pytest.raises(C.CatalogError):
        C.load_soil(CONFIG, ad_class="AD9")
    with pytest.raises(C.CatalogError):
        C.load_soil(CONFIG, texture="pedregosa")


def test_random_soils_enumerate_and_verify(cat):
    rnd = random.Random(7)
    for _ in range(4):
        s = C.load_soil(
            CONFIG,
            ad_class=f"AD{rnd.randint(1, 6)}",
            texture=rnd.choice(["arenosa", "media", "argilosa"]),
        )
        plans, _ = C.enumerate_plans(cat, s)
        assert plans
        for p in plans[::25]:
            assert C.verify_plan(cat, s, p) == []
