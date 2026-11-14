// AI-assisted (OpenBot agent)
//! Portes 1-para-1 de `tests/test_catalog.py` (32 testes Python): calendário, enumeração ZARC <= 20%,
//! IDs, verificador, INVIÁVEL, P0, falhas explícitas e verificação sobre todos os 18 solos.

mod common;

use std::collections::BTreeMap;

use common::*;
use fieldshift_model::catalog::*;

fn raw_zarc(c: &Config, name: &str, cod_solo: &str) -> Vec<i32> {
    let text = String::from_utf8(c.zarc_csv.clone()).unwrap();
    let text = text.trim_start_matches('\u{feff}');
    let mut lines = text.lines();
    let head: Vec<&str> = lines.next().unwrap().split(';').collect();
    let col = |n: &str| head.iter().position(|h| *h == n).unwrap();
    let hits: Vec<Vec<&str>> = lines
        .map(|l| l.split(';').collect::<Vec<_>>())
        .filter(|r| {
            r[col("Nome_cultura")] == name
                && r[col("Cod_Ciclo")] == "20"
                && r[col("Cod_Outros_Manejos")] == "1"
                && r[col("Cod_Solo")] == cod_solo
                && r[col("geocodigo")] == "5218805"
        })
        .collect();
    assert_eq!(hits.len(), 1, "{name}/{cod_solo}");
    (1..=36)
        .map(|d| hits[0][col(&format!("dec{d}"))].parse().unwrap())
        .collect()
}

#[test]
fn test_calendar_day_index() {
    assert_eq!(md_to_t(9, 1).unwrap(), 0);
    assert_eq!(md_to_t(10, 1).unwrap(), 30);
    assert_eq!(md_to_t(1, 1).unwrap(), 122);
    assert_eq!(md_to_t(8, 31).unwrap(), 364);
    assert!(
        md_to_t(2, 29).is_err(),
        "29/fev não existe no calendário de 365 dias"
    );
    assert_eq!(t_to_md(40), (10, 11));
    assert_eq!(t_to_md(40 + 365), (10, 11));
    assert_eq!(t_label(147), "26/jan");
    assert_eq!(t_label(147 + 365), "26/jan (ano 2)");
}

#[test]
fn test_decendios() {
    assert_eq!((dec_start_t(25).unwrap(), dec_end_t(25).unwrap()), (0, 9));
    assert_eq!(dec_start_t(29).unwrap(), 40);
    assert_eq!(dec_end_t(6).unwrap() - dec_start_t(6).unwrap() + 1, 8);
    assert_eq!(dec_end_t(3).unwrap() - dec_start_t(3).unwrap() + 1, 11);
    assert_eq!(t_to_dec(147), 3);
    assert_eq!([25, 36, 1, 24].map(dec_ordinal), [0, 11, 12, 35]);
    let mut covered: Vec<i32> = Vec::new();
    for d in 1..=36 {
        covered.extend(dec_start_t(d).unwrap()..=dec_end_t(d).unwrap());
    }
    covered.sort_unstable();
    assert_eq!(covered, (0..365).collect::<Vec<_>>());
}

type J2Map = BTreeMap<&'static str, (i32, i32)>;
type ExpectedFirstSeason = ((&'static str, i32), (i32, J2Map));
type ObservedFirstSeason = BTreeMap<(String, i32), (i32, BTreeMap<String, (i32, i32)>)>;

fn expected_j1_j2() -> Vec<ExpectedFirstSeason> {
    let three = |a: i32, b: i32| -> J2Map {
        BTreeMap::from([
            ("milho_2a", (a, b)),
            ("milho_2a_braquiaria", (a, b + 5)),
            ("sorgo_2a", (a, b)),
        ])
    };
    let sorgo = |a: i32, b: i32| -> J2Map { BTreeMap::from([("sorgo_2a", (a, b))]) };
    let none = BTreeMap::new;
    vec![
        (("soja", 28), (137, three(137, 254))),
        (("soja", 29), (147, three(147, 264))),
        (("soja", 30), (157, three(157, 274))),
        (("soja", 31), (168, sorgo(168, 285))),
        (("soja", 32), (178, sorgo(178, 295))),
        (("soja", 33), (188, sorgo(188, 305))),
        (("soja", 34), (198, sorgo(198, 315))),
        (("soja", 35), (208, none())),
        (("soja", 36), (218, none())),
        (("milho_1a", 30), (157, three(157, 274))),
        (("milho_1a", 31), (168, sorgo(168, 285))),
        (("milho_1a", 32), (178, sorgo(178, 295))),
        (("milho_1a", 33), (188, sorgo(188, 305))),
        (("milho_1a", 34), (198, sorgo(198, 315))),
        (("milho_1a", 35), (208, none())),
        (("milho_1a", 36), (218, none())),
    ]
}

#[test]
fn test_verification_table_methods() {
    let c = config();
    let cat = catalog(&c);
    let soil = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &soil, 0.20).unwrap();
    let mut got = ObservedFirstSeason::new();
    for p in &plans {
        let (o1, o2) = (&p.occupations[0], &p.occupations[1]);
        let key = (cat.crops[o1.crop].key.clone(), o1.dec.unwrap());
        let entry = got
            .entry(key)
            .or_insert((o1.harvest_t.unwrap(), BTreeMap::new()));
        if !o2.is_fallow() {
            entry.1.insert(
                cat.crops[o2.crop].key.clone(),
                (o2.sow_t.unwrap(), o2.harvest_t.unwrap()),
            );
        }
    }
    let exp = expected_j1_j2();
    assert_eq!(got.len(), exp.len());
    for ((crop, dec), (h1, opts)) in exp {
        let (gh, go) = &got[&(crop.to_string(), dec)];
        assert_eq!(*gh, h1, "{crop} dec{dec}");
        let want: BTreeMap<String, (i32, i32)> =
            opts.into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        assert_eq!(go, &want, "{crop} dec{dec}");
    }
}

#[test]
fn test_viable_options_per_j1_only_pousio_when_no_second_crop() {
    let c = config();
    let cat = catalog(&c);
    let (plans, _) = enumerate_plans(&cat, &soil(&c, None, None), 0.20).unwrap();
    for p in &plans {
        let (o1, o2) = (&p.occupations[0], &p.occupations[1]);
        if o2.is_fallow() {
            assert!(
                [35, 36].contains(&o1.dec.unwrap()),
                "pousio com 2ª safra viável: {:?}",
                p.id
            );
        }
    }
}

fn check_size_of_x(ad: &str, expected: usize) {
    let c = config();
    let cat = catalog(&c);
    let (plans, _) = enumerate_plans(&cat, &soil(&c, Some(ad), None), 0.20).unwrap();
    assert_eq!(plans.len(), expected, "{ad}");
}

#[test]
fn test_size_of_x_matches_methods_table_ad1() {
    check_size_of_x("AD1", 169);
}

#[test]
fn test_size_of_x_matches_methods_table_ad2() {
    check_size_of_x("AD2", 289);
}

#[test]
fn test_size_of_x_matches_methods_table_ad3() {
    check_size_of_x("AD3", 400);
}

#[test]
fn test_size_of_x_matches_methods_table_ad4() {
    check_size_of_x("AD4", 576);
}

#[test]
fn test_size_of_x_matches_methods_table_ad5() {
    check_size_of_x("AD5", 729);
}

#[test]
fn test_size_of_x_matches_methods_table_ad6() {
    check_size_of_x("AD6", 961);
}

#[test]
fn test_fallow_report_default_soil() {
    let c = config();
    let cat = catalog(&c);
    let (plans, _) = enumerate_plans(&cat, &soil(&c, None, None), 0.20).unwrap();
    let want: BTreeMap<String, i64> = [
        ("total", 576),
        ("com_pousio", 176),
        ("pousio_J2", 96),
        ("pousio_J4", 96),
        ("pousio_J2_e_J4", 16),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    assert_eq!(fallow_summary(&plans), want);
}

#[test]
fn test_every_sowing_is_exactly_20_percent_per_independent_reading() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    for p in &plans {
        for o in p.occupations.iter().filter(|o| !o.is_fallow()) {
            let crop = &cat.crops[o.crop];
            let cod = s.cod_solo(crop.zarc_soil_key.as_deref().unwrap()).unwrap();
            let row = raw_zarc(&c, crop.zarc_name.as_deref().unwrap(), &cod);
            assert_eq!(
                row[(t_to_dec(o.sow_t.unwrap()) - 1) as usize],
                20,
                "{:?}",
                p.id
            );
        }
    }
}

#[test]
fn test_ids_unique_sorted_and_rederivable() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    assert!(
        plans.windows(2).all(|w| w[0].id < w[1].id),
        "IDs únicos e ordenados"
    );
    for p in plans.iter().step_by(15) {
        let o = &p.occupations;
        let ordn = |x: &Occupation| x.dec.map_or(-1, dec_ordinal);
        assert_eq!(
            p.id,
            [
                o[0].crop as i32,
                ordn(&o[0]),
                o[1].crop as i32,
                o[2].crop as i32,
                ordn(&o[2]),
                o[3].crop as i32
            ]
        );
    }
}

#[test]
fn test_ids_do_not_depend_on_enumeration_order() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    let mut cat2 = cat.clone();
    for w in ["J1", "J2"] {
        cat2.windows.get_mut(w).unwrap().crops.reverse();
    }
    let (plans2, _) = enumerate_plans(&cat2, &s, 0.20).unwrap();
    assert_eq!(plans2, plans);
}

#[test]
fn test_limit_cannot_be_raised() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    assert!(enumerate_plans(&cat, &s, 0.30).is_err());
    let cat_text = String::from_utf8(c.catalog_toml.clone())
        .unwrap()
        .replace("risk_limit_percent = 20", "risk_limit_percent = 30");
    let err = load_catalog(&CatalogSources {
        catalog_toml: cat_text.as_bytes(),
        soil_toml: &c.soil_toml,
        zarc_csv: &c.zarc_csv,
    })
    .unwrap_err();
    assert!(err.0.contains("risk_limit_percent"), "{err}");
}

#[test]
fn test_stricter_limit_gives_inviavel_not_relaxation() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, rej) = enumerate_plans(&cat, &s, 0.10).unwrap();
    assert!(plans.is_empty());
    let diag = diagnose_infeasible(&rej);
    assert_eq!(diag["status"], "INVIAVEL");
    assert!(diag["n_rejections"].as_i64().unwrap() > 0);
    assert!(diag["rejections_by_reason"]
        .as_object()
        .unwrap()
        .keys()
        .any(|k| k.contains("ZARC_RISCO_ACIMA")));
}

#[test]
fn test_empty_x_is_inviavel_with_diagnosis() {
    let c = config();
    let mut cat = catalog(&c);
    for row in cat.zarc.values_mut() {
        for v in row.iter_mut() {
            if *v == 20 {
                *v = 30;
            }
        }
    }
    let (plans, rej) = enumerate_plans(&cat, &soil(&c, None, None), 0.20).unwrap();
    assert!(plans.is_empty());
    let diag = diagnose_infeasible(&rej);
    assert!(diag["message"]
        .as_str()
        .unwrap()
        .contains("não foram alterados"));
    assert!(rej
        .iter()
        .any(|r| r.reason == "ZARC_RISCO_ACIMA" && r.crop == "soja"));
}

#[test]
fn test_rejections_carry_reasons() {
    let c = config();
    let cat = catalog(&c);
    let (_, rej) = enumerate_plans(&cat, &soil(&c, None, None), 0.20).unwrap();
    let by = |w: &str, crop: &str, dec: i32| {
        rej.iter()
            .find(|r| r.window == w && r.crop == crop && r.dec == Some(dec))
            .unwrap()
    };
    assert_eq!(by("J1", "soja", 27).reason, "ZARC_RISCO_ACIMA");
    assert!(by("J1", "soja", 27).detail.contains("40"));
    assert_eq!(by("J1", "milho_1a", 29).reason, "ZARC_RISCO_ACIMA");
    assert_eq!(by("J1", "soja", 25).reason, "ZARC_NAO_INDICADO");
    assert!(rej
        .iter()
        .any(|r| r.reason == "SEM_DECENDIO_ZARC_APOS_COLHEITA"));
    assert_eq!(summarize_rejections(&rej)["J1:soja:ZARC_RISCO_ACIMA"], 1);
}

#[test]
fn test_legal_window_excludes_dec1_for_soja() {
    let c = config();
    let mut cat = catalog(&c);
    cat.windows.get_mut("J1").unwrap().decendios.insert(0, 1);
    let (plans, rej) = enumerate_plans(&cat, &soil(&c, None, None), 0.20).unwrap();
    assert_eq!(plans.len(), 576);
    let hit: Vec<_> = rej
        .iter()
        .filter(|r| r.crop == "soja" && r.dec == Some(1))
        .collect();
    assert!(!hit.is_empty() && hit[0].reason == "JANELA_LEGAL");
}

fn find<'a>(cat: &Catalog, plans: &'a [Plan], crops: [&str; 4], decs: (i32, i32)) -> &'a Plan {
    plans
        .iter()
        .find(|p| {
            let keys: Vec<&str> = p
                .occupations
                .iter()
                .map(|o| cat.crops[o.crop].key.as_str())
                .collect();
            keys == crops && (p.occupations[0].dec.unwrap(), p.occupations[2].dec.unwrap()) == decs
        })
        .expect("plano")
}

#[test]
fn test_all_enumerated_plans_pass_verifier() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    for p in &plans {
        assert!(
            verify_plan(&cat, &s, p, 0.20).unwrap().is_empty(),
            "{:?}",
            p.id
        );
    }
}

#[test]
fn test_verifier_rejects_30_percent_decendio() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    let p = find(
        &cat,
        &plans,
        ["soja", "milho_2a", "soja", "milho_2a"],
        (29, 29),
    )
    .clone();
    let mut bad = p.clone();
    let s27 = dec_start_t(27).unwrap();
    bad.occupations[0] = Occupation {
        sow_t: Some(s27),
        emerge_t: Some(s27 + 7),
        harvest_t: Some(s27 + 107),
        dec: Some(27),
        ..p.occupations[0]
    };
    assert!(verify_plan(&cat, &s, &bad, 0.20)
        .unwrap()
        .iter()
        .any(|e| e.contains("ZARC 40%")));
}

#[test]
fn test_verifier_rejects_overlap_j1_j2() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    let p = find(
        &cat,
        &plans,
        ["soja", "milho_2a", "soja", "milho_2a"],
        (29, 29),
    )
    .clone();
    let mut bad = p.clone();
    let early = p.occupations[1].sow_t.unwrap() - 5;
    bad.occupations[1] = Occupation {
        sow_t: Some(early),
        emerge_t: Some(early + 7),
        harvest_t: Some(early + 117),
        dec: Some(t_to_dec(early)),
        ..p.occupations[1]
    };
    assert!(verify_plan(&cat, &s, &bad, 0.20)
        .unwrap()
        .iter()
        .any(|e| e.contains("sobreposição")));
}

#[test]
fn test_verifier_rejects_overlap_j2_j3_and_wrong_window() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    let p = find(
        &cat,
        &plans,
        ["soja", "milho_2a", "soja", "milho_2a"],
        (29, 29),
    )
    .clone();
    let mut bad = p.clone();
    bad.occupations[2] = Occupation {
        sow_t: Some(250),
        emerge_t: Some(257),
        harvest_t: Some(357),
        dec: Some(t_to_dec(250)),
        ..p.occupations[2]
    };
    assert!(verify_plan(&cat, &s, &bad, 0.20)
        .unwrap()
        .iter()
        .any(|e| e.contains("J3")));
    let mut bad = p;
    bad.occupations[1].crop = cat.index_of("soja").unwrap();
    assert!(verify_plan(&cat, &s, &bad, 0.20)
        .unwrap()
        .iter()
        .any(|e| e.contains("não pertence")));
}

#[test]
fn test_verifier_rejects_late_second_crop() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    let p = find(
        &cat,
        &plans,
        ["soja", "milho_2a", "soja", "milho_2a"],
        (29, 29),
    )
    .clone();
    let mut bad = p.clone();
    let late = dec_start_t(5).unwrap();
    bad.occupations[1] = Occupation {
        sow_t: Some(late),
        emerge_t: Some(late + 7),
        harvest_t: Some(late + 117),
        dec: Some(5),
        ..p.occupations[1]
    };
    assert!(verify_plan(&cat, &s, &bad, 0.20)
        .unwrap()
        .iter()
        .any(|e| e.contains("ZARC")));
}

#[test]
fn test_default_p0_is_viable_ad4() {
    let c = config();
    let cat = catalog(&c);
    assert_eq!(
        cat.default_p0,
        PlanSpec {
            j1: "soja".into(),
            dec1: 29,
            j2: "milho_2a".into(),
            j3: "soja".into(),
            dec3: 29,
            j4: "milho_2a".into()
        }
    );
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    let (p, why) = resolve_spec(&cat, &s, &cat.default_p0, &plans, 0.20).unwrap();
    assert!(why.is_empty());
    let d = describe_plan(&cat, &p.unwrap());
    let occ = d["occupations"].as_array().unwrap();
    assert_eq!(
        (
            occ[0]["sow"].as_str().unwrap(),
            occ[0]["harvest_t"].as_i64().unwrap()
        ),
        ("11/out", 147)
    );
    assert_eq!(
        (
            occ[1]["sow"].as_str().unwrap(),
            occ[1]["harvest_t"].as_i64().unwrap()
        ),
        ("26/jan", 264)
    );
}

#[test]
fn test_default_p0_inviavel_in_ad1() {
    let c = config();
    let cat = catalog(&c);
    let s1 = soil(&c, Some("AD1"), None);
    let (plans1, _) = enumerate_plans(&cat, &s1, 0.20).unwrap();
    let (p, why) = resolve_spec(&cat, &s1, &cat.default_p0, &plans1, 0.20).unwrap();
    assert!(p.is_none() && why.iter().any(|r| r.contains("soja") && r.contains("ZARC")));
}

#[test]
fn test_p0_with_fallow_when_second_crop_viable_is_outside_x() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    let fallow_spec = PlanSpec {
        j1: "soja".into(),
        dec1: 29,
        j2: "pousio".into(),
        j3: "soja".into(),
        dec3: 29,
        j4: "milho_2a".into(),
    };
    let (p, why) = resolve_spec(&cat, &s, &fallow_spec, &plans, 0.20).unwrap();
    assert!(p.is_none() && why.iter().any(|r| r.contains("pousio não permitido")));
}

#[test]
fn test_p0_second_crop_after_window_is_outside_x() {
    let c = config();
    let cat = catalog(&c);
    let s = soil(&c, None, None);
    let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
    let late = PlanSpec {
        j1: "soja".into(),
        dec1: 31,
        j2: "milho_2a".into(),
        j3: "soja".into(),
        dec3: 29,
        j4: "milho_2a".into(),
    };
    let (p, why) = resolve_spec(&cat, &s, &late, &plans, 0.20).unwrap();
    assert!(p.is_none() && why.iter().any(|r| r.contains("milho_2a inviável")));
    let unknown = PlanSpec {
        j1: "trigo".into(),
        ..cat.default_p0.clone()
    };
    let (p, why) = resolve_spec(&cat, &s, &unknown, &plans, 0.20).unwrap();
    assert!(p.is_none() && why[0].starts_with("plano inválido"));
}

#[test]
fn test_zarc_hash_mismatch_fails_loudly() {
    let c = config();
    let mut csv = c.zarc_csv.clone();
    csv.push(b'\n');
    let err = load_catalog(&CatalogSources {
        catalog_toml: &c.catalog_toml,
        soil_toml: &c.soil_toml,
        zarc_csv: &csv,
    })
    .unwrap_err();
    assert!(err.0.contains("SHA-256"), "{err}");
}

#[test]
fn test_duplicate_zarc_row_fails_loudly() {
    let c = config();
    let text = String::from_utf8(c.zarc_csv.clone()).unwrap();
    let soja = text
        .lines()
        .skip(1)
        .find(|l| l.starts_with("Soja;") && l.contains(";20;14;"))
        .unwrap()
        .to_string();
    let mut dup = text.trim_end_matches('\n').to_string();
    dup.push('\n');
    dup.push_str(&soja);
    dup.push('\n');
    let new_sha = fieldshift_model::compat::sha256_hex(dup.as_bytes());
    let cat_text = String::from_utf8(c.catalog_toml.clone()).unwrap().replace(
        "88f24986b6063e0eaba7bd0cd83f97d9a623f5c5f9ada156711e318b9e3218ad",
        &new_sha,
    );
    let err = load_catalog(&CatalogSources {
        catalog_toml: cat_text.as_bytes(),
        soil_toml: &c.soil_toml,
        zarc_csv: dup.as_bytes(),
    })
    .unwrap_err();
    assert!(err.0.contains("duplicada"), "{err}");
}

#[test]
fn test_unknown_soil_values_fail() {
    let c = config();
    assert!(load_soil(&c.soil_toml, Some("AD9"), None).is_err());
    assert!(load_soil(&c.soil_toml, None, Some("pedregosa")).is_err());
}

#[test]
fn test_random_soils_enumerate_and_verify() {
    let c = config();
    let cat = catalog(&c);
    for ad in ["AD1", "AD2", "AD3", "AD4", "AD5", "AD6"] {
        for tx in ["arenosa", "media", "argilosa"] {
            let s = soil(&c, Some(ad), Some(tx));
            let (plans, _) = enumerate_plans(&cat, &s, 0.20).unwrap();
            assert!(!plans.is_empty(), "{ad}/{tx}");
            for p in &plans {
                assert!(
                    verify_plan(&cat, &s, p, 0.20).unwrap().is_empty(),
                    "{ad}/{tx}: {:?}",
                    p.id
                );
            }
        }
    }
}
