// AI-assisted (OpenBot agent)
//! E3: o catálogo em Rust reproduz o Python (`16cc391`) — mesmos IDs, ordem, datas, rejeições, P0.

mod common;

use common::*;
use fieldshift_model::catalog::*;
use serde_json::{json, Value};

const CASES: [(&str, &str, &str); 8] = [
    ("AD1_argilosa", "AD1", "argilosa"),
    ("AD2_argilosa", "AD2", "argilosa"),
    ("AD3_argilosa", "AD3", "argilosa"),
    ("AD4_arenosa", "AD4", "arenosa"),
    ("AD4_argilosa", "AD4", "argilosa"),
    ("AD4_media", "AD4", "media"),
    ("AD5_argilosa", "AD5", "argilosa"),
    ("AD6_argilosa", "AD6", "argilosa"),
];

#[test]
fn plans_match_python_for_all_soils() {
    let c = config();
    let cat = catalog(&c);
    let manifest = fixture("MANIFEST.json");
    for (name, ad, tx) in CASES {
        let fx = fixture(&format!("catalog_{name}.json"));
        let soil = soil(&c, Some(ad), Some(tx));
        let (plans, rej) = enumerate_plans(&cat, &soil, 0.20).expect("enumerar");
        assert_eq!(
            plans.len() as i64,
            manifest["plan_counts"][name].as_i64().unwrap(),
            "{name}: contagem"
        );
        assert_eq!(
            plans.len() as i64,
            fx["n_plans"].as_i64().unwrap(),
            "{name}"
        );
        let fplans = fx["plans"].as_array().unwrap();
        assert_eq!(plans.len(), fplans.len());
        for (i, (p, fp)) in plans.iter().zip(fplans).enumerate() {
            assert_eq!(p.id.to_vec(), ids_of(&fp["id"]), "{name}[{i}]: ID/ordem");
            let sig: Vec<i32> = p.signature().iter().map(|x| *x as i32).collect();
            assert_eq!(sig, ids_of(&fp["signature"]), "{name}[{i}]: assinatura");
            let fw: Vec<&str> = fp["fallow_windows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap())
                .collect();
            assert_eq!(p.fallow_windows(), fw, "{name}[{i}]: pousio");
            for (o, fo) in p
                .occupations
                .iter()
                .zip(fp["occupations"].as_array().unwrap())
            {
                assert_eq!(o.window_name(), fo["window"].as_str().unwrap());
                assert_eq!(o.crop as i64, fo["crop"].as_i64().unwrap());
                assert_eq!(
                    o.sow_t.map(i64::from),
                    fo["sow_t"].as_i64(),
                    "{name}[{i}] sow_t"
                );
                assert_eq!(
                    o.emerge_t.map(i64::from),
                    fo["emerge_t"].as_i64(),
                    "{name}[{i}] emerge_t"
                );
                assert_eq!(
                    o.harvest_t.map(i64::from),
                    fo["harvest_t"].as_i64(),
                    "{name}[{i}] harvest_t"
                );
                assert_eq!(o.dec.map(i64::from), fo["dec"].as_i64(), "{name}[{i}] dec");
            }
        }
        let frej = fx["rejections"].as_array().unwrap();
        assert_eq!(rej.len(), frej.len(), "{name}: nº de rejeições");
        for (r, fr) in rej.iter().zip(frej) {
            assert_eq!(&r.to_value(), fr, "{name}: rejeição");
        }
        let fs: std::collections::BTreeMap<String, i64> = fx["fallow_summary"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (k.clone(), v.as_i64().unwrap()))
            .collect();
        assert_eq!(fallow_summary(&plans), fs, "{name}: resumo de pousio");
        // P0
        let (p0, reasons) = resolve_spec(&cat, &soil, &cat.default_p0, &plans, 0.20).unwrap();
        let fp0 = &fx["p0_plan_id"];
        match p0 {
            Some(p) => assert_eq!(p.id.to_vec(), ids_of(fp0), "{name}: P0"),
            None => assert!(fp0.is_null(), "{name}: P0 deveria ser inviável"),
        }
        let want: Vec<String> = fx["p0_reasons"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect();
        assert_eq!(reasons, want, "{name}: motivos do P0");
        assert_eq!(
            fx["p0_spec"],
            json!({"dec1": 29, "dec3": 29, "j1": "soja", "j2": "milho_2a", "j3": "soja", "j4": "milho_2a"})
        );
        assert_eq!(cat.default_p0.to_value().as_object().unwrap().len(), 6);
    }
}

#[test]
fn verifier_accepts_every_enumerated_plan_in_every_soil() {
    let c = config();
    let cat = catalog(&c);
    for (_, ad, tx) in CASES {
        let soil = soil(&c, Some(ad), Some(tx));
        let (plans, _) = enumerate_plans(&cat, &soil, 0.20).unwrap();
        for p in &plans {
            assert!(
                verify_plan(&cat, &soil, p, 0.20).unwrap().is_empty(),
                "{ad}/{tx}: {:?}",
                p.id
            );
        }
    }
}

#[test]
fn fixture_diagnosis_is_null_when_feasible() {
    for (name, _, _) in CASES {
        assert_eq!(
            fixture(&format!("catalog_{name}.json"))["diagnosis_if_empty"],
            Value::Null
        );
    }
}
