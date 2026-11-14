// AI-assisted (OpenBot agent)
//! E6: avaliação Rust contra o oráculo Python (16cc391). Os componentes g1–g3 vêm do oráculo
//! (indexados pelo CONTEÚDO do cenário que o Rust monta), isolando a avaliação dos indicadores.
//! O teste com os indicadores Rust reais fica em `differential_eval_model.rs`.
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use fieldshift_eval::evaluation::{
    array_hash, construction_dry_totals, construction_precip_fingerprint, dry_threshold,
    evaluate_plans, mean_climate, split_scenarios, ClimateDaily, EvalError, EvalInput,
    EvalProtocol, LossModel, PlanInfo, Scenario,
};
use fieldshift_eval::npcompat::sha256_hex;
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/reference")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(p).expect("fixture")).expect("json")
}

fn fvec(v: &Value) -> Vec<f64> {
    v.as_array()
        .expect("array")
        .iter()
        .map(|x| x.as_f64().expect("f64"))
        .collect()
}

pub fn climate_from_fixture() -> ClimateDaily {
    let f = fixture("climate_daily.json");
    let dates = f["dates"]
        .as_array()
        .expect("dates")
        .iter()
        .map(|d| {
            let s = d.as_str().expect("str");
            (
                s[0..4].parse().expect("y"),
                s[5..7].parse().expect("m"),
                s[8..10].parse().expect("d"),
            )
        })
        .collect();
    ClimateDaily {
        dates,
        prectotcorr: fvec(&f["PRECTOTCORR"]),
        t2m: fvec(&f["T2M"]),
        t2m_max: fvec(&f["T2M_MAX"]),
        t2m_min: fvec(&f["T2M_MIN"]),
    }
}

pub fn plans_from_fixture(name: &str) -> Vec<PlanInfo> {
    fixture(name)["plans"]
        .as_array()
        .expect("plans")
        .iter()
        .map(|p| PlanInfo {
            id: p["id"]
                .as_array()
                .expect("id")
                .iter()
                .map(|x| x.as_i64().expect("i"))
                .collect(),
            genes: p["occupations"]
                .as_array()
                .expect("occ")
                .iter()
                .map(|o| (o["crop"].as_i64().expect("crop"), o["sow_t"].as_i64()))
                .collect(),
        })
        .collect()
}

/// Modelo de perda de TESTE: devolve os componentes do oráculo para o cenário de mesmo conteúdo.
struct OracleComponents {
    by_key: HashMap<String, Vec<[f64; 3]>>,
}

impl OracleComponents {
    fn load() -> Self {
        let f = fixture("components_by_content.json");
        let by_key = f["scenarios"]
            .as_object()
            .expect("obj")
            .iter()
            .map(|(k, v)| {
                let g = v["g"]
                    .as_array()
                    .expect("g")
                    .iter()
                    .map(|r| {
                        let r = fvec(r);
                        [r[0], r[1], r[2]]
                    })
                    .collect();
                (k.clone(), g)
            })
            .collect();
        OracleComponents { by_key }
    }
}

impl LossModel for OracleComponents {
    fn fit(&mut self, construction: &[Scenario]) -> Result<String, EvalError> {
        Ok(construction_precip_fingerprint(construction))
    }
    fn components(&mut self, plan: usize, sc: &Scenario) -> Result<[f64; 3], EvalError> {
        let g = self
            .by_key
            .get(&sc.content_key())
            .ok_or_else(|| EvalError(format!("cenário sem oráculo: {}", sc.label)))?;
        Ok(g[plan])
    }
}

fn diff(path: &str, a: &Value, b: &Value, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                diff(
                    &format!("{path}.{k}"),
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                diff(&format!("{path}[{i}]"), p, q, out);
            }
        }
        _ if a != b => out.push(format!("{path}: rust={a} python={b}")),
        _ => {}
    }
}

fn policy_ids(rr: &Value) -> BTreeMap<String, Option<Vec<i64>>> {
    ["P0", "P1", "P1b", "P2"]
        .iter()
        .map(|p| {
            let id = rr["policies"][p]["plan"]["id"]
                .as_array()
                .map(|a| a.iter().map(|x| x.as_i64().expect("i")).collect());
            ((*p).to_string(), id)
        })
        .collect()
}

#[test]
fn split_sbar_and_dry_threshold_match_oracle() {
    let c = climate_from_fixture();
    let p = EvalProtocol::default();
    let s = split_scenarios(&c, &p).expect("split");
    let ei = fixture("eval_intermediates.json");
    let labels = |v: &Value| -> Vec<String> {
        v.as_array()
            .expect("a")
            .iter()
            .map(|x| x.as_str().expect("s").to_string())
            .collect()
    };
    assert_eq!(
        s.construction
            .iter()
            .map(|x| x.label.clone())
            .collect::<Vec<_>>(),
        labels(&ei["construction_labels"])
    );
    assert_eq!(
        s.test.iter().map(|x| x.label.clone()).collect::<Vec<_>>(),
        labels(&ei["test_labels"])
    );
    assert!(s.excluded.is_empty());
    // P25 e totais out–mar: bit a bit
    assert_eq!(
        construction_dry_totals(&s.construction_years, &p),
        fvec(&ei["construction_dry_season_totals"])
    );
    assert_eq!(
        dry_threshold(&s.construction_years, &p),
        ei["dry_threshold_mm"].as_f64().expect("f")
    );
    let cy: Vec<_> = s.construction_years.iter().collect();
    let sb = mean_climate(&cy).expect("sbar");
    assert_eq!(
        array_hash(&[&sb.precip, &sb.tmax, &sb.tmin]),
        ei["mean_climate_hash"].as_str().expect("s")
    );
    let sc = fixture("scenarios.json");
    assert_eq!(
        sb.content_key(),
        sc["sbar_sha256"].as_str().expect("s"),
        "s̄ igual ao do otimizador"
    );
    assert_eq!(
        construction_precip_fingerprint(&s.construction),
        ei["params_hash"].as_str().expect("s")
    );
    // todos os 42 cenários têm o mesmo conteúdo bit a bit que o Python
    let oracle = OracleComponents::load();
    for x in s.construction.iter().chain(&s.test) {
        assert!(
            oracle.by_key.contains_key(&x.content_key()),
            "conteúdo divergente: {}",
            x.label
        );
    }
}

pub fn python_x_hash(plans: &[PlanInfo]) -> String {
    let body: Vec<String> = plans
        .iter()
        .map(|p| {
            format!(
                "[{}]",
                p.id.iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
        .collect();
    sha256_hex(format!("[{}]", body.join(", ")).as_bytes())
}

#[test]
fn full_evaluation_report_equals_python_and_hash_504c28e8() {
    let c = climate_from_fixture();
    let plans = plans_from_fixture("catalog_AD4_argilosa.json");
    let rr = fixture("golden_run_result.json");
    let p = EvalProtocol::default();
    let input = EvalInput {
        plans: &plans,
        policy_ids: policy_ids(&rr),
        p0_status: None,
        climate: &c,
        weights: None,
        check_plan: None,
        run_grid: true,
        optimizer_params_hash: rr["climatology"]["precip_sha256"]
            .as_str()
            .map(String::from),
    };
    let mut model = OracleComponents::load();
    let mut report = evaluate_plans(&input, &mut model, &p).expect("evaluate");
    // como scripts/run_evaluation.py, depois de evaluate()
    report.audit_set(
        "x_hash_match",
        json!(python_x_hash(&plans) == rr["x_hash"].as_str().expect("s")),
    );
    let s = split_scenarios(&c, &p).expect("split");
    let sb = mean_climate(&s.construction_years.iter().collect::<Vec<_>>()).expect("sbar");
    report.audit_set(
        "sbar_hash_match",
        json!(sb.content_key() == rr["climatology"]["sbar_sha256"].as_str().expect("s")),
    );
    report.audit_set("optimizer_status", json!(rr["status"]));

    let want = fixture("eval_intermediates.json")["report_dict"].clone();
    let mut d = Vec::new();
    diff("$", &report.to_value(), &want, &mut d);
    assert!(
        d.is_empty(),
        "{} divergências; primeiras: {:#?}",
        d.len(),
        &d[..d.len().min(15)]
    );
    assert_eq!(report.outcome, "NEGATIVO");
    let golden = fixture("golden_eval_report.json");
    assert_eq!(
        report.result_hash(),
        golden["result_hash"].as_str().expect("s")
    );
    assert_eq!(
        report.result_hash(),
        "504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18"
    );
}

#[test]
fn markdown_equals_golden_python_markdown() {
    let c = climate_from_fixture();
    let plans = plans_from_fixture("catalog_AD4_argilosa.json");
    let rr = fixture("golden_run_result.json");
    let p = EvalProtocol::default();
    let input = EvalInput {
        plans: &plans,
        policy_ids: policy_ids(&rr),
        p0_status: None,
        climate: &c,
        weights: None,
        check_plan: None,
        run_grid: true,
        optimizer_params_hash: rr["climatology"]["precip_sha256"]
            .as_str()
            .map(String::from),
    };
    let mut report = evaluate_plans(&input, &mut OracleComponents::load(), &p).expect("evaluate");
    report.audit_set("x_hash_match", json!(true));
    report.audit_set("sbar_hash_match", json!(true));
    report.audit_set("optimizer_status", json!("OK"));
    let golden = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../reports/20261114T184118Z/eval_report.md"),
    )
    .expect("md");
    let got = report.to_markdown();
    for (i, (a, b)) in got.lines().zip(golden.lines()).enumerate() {
        assert_eq!(a, b, "linha {}", i + 1);
    }
    assert_eq!(got, golden);
}

#[path = "../../fieldshift-model/tests/common/mod.rs"]
mod model_common;

#[test]
fn real_rust_model_recomputes_complete_negative_report() {
    use fieldshift_eval::adapter::{plan_checker, plan_infos, IndicatorLossModel};
    use fieldshift_model::{
        catalog::enumerate_plans,
        optimization::{run, RunOptions},
    };
    let cfg = model_common::config();
    let cat = model_common::catalog(&cfg);
    let soil = model_common::soil(&cfg, None, None);
    let params = model_common::params(&cfg, &cat);
    let (plans, _) = enumerate_plans(&cat, &soil, 0.20).expect("plans");
    let infos = plan_infos(&plans);
    let golden = fixture("golden_run_result.json");
    let rr = run(
        &model_common::climate(),
        &cat,
        &soil,
        &params,
        golden["snapshot_id"].as_str().expect("snapshot"),
        &RunOptions::default(),
    )
    .expect("run")
    .to_value(false);
    let c = climate_from_fixture();
    let checker = plan_checker(&cat, &soil, &plans);
    let input = EvalInput {
        plans: &infos,
        policy_ids: policy_ids(&rr),
        p0_status: None,
        climate: &c,
        weights: None,
        check_plan: Some(&checker),
        run_grid: true,
        optimizer_params_hash: rr["climatology"]["precip_sha256"]
            .as_str()
            .map(String::from),
    };
    let mut model = IndicatorLossModel::new(&plans, &params);
    let mut report =
        evaluate_plans(&input, &mut model, &EvalProtocol::default()).expect("real evaluation");
    report.audit_set(
        "x_hash_match",
        json!(python_x_hash(&infos) == rr["x_hash"].as_str().expect("hash")),
    );
    let split = split_scenarios(&c, &EvalProtocol::default()).expect("split");
    let sb = mean_climate(&split.construction_years.iter().collect::<Vec<_>>()).expect("mean");
    report.audit_set(
        "sbar_hash_match",
        json!(sb.content_key() == rr["climatology"]["sbar_sha256"].as_str().expect("hash")),
    );
    report.audit_set("optimizer_status", json!(rr["status"]));
    let mut d = Vec::new();
    diff(
        "$",
        &report.to_value(),
        &fixture("eval_intermediates.json")["report_dict"],
        &mut d,
    );
    assert!(
        d.is_empty(),
        "{} differences: {:?}",
        d.len(),
        &d[..d.len().min(15)]
    );
    assert_eq!(report.outcome, "NEGATIVO");
    assert_eq!(
        report.result_hash(),
        "504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18"
    );
}

#[test]
fn m2_missing_t2m_excludes_both_overlapping_pairs() {
    let mut c = climate_from_fixture();
    let i = c
        .dates
        .iter()
        .position(|d| *d == (1990, 1, 15))
        .expect("date");
    c.t2m[i] = -999.0;
    let s = split_scenarios(&c, &EvalProtocol::default()).expect("split");
    assert_eq!(s.construction.len(), 26);
    assert!(!s
        .construction
        .iter()
        .any(|s| s.start == 1988 || s.start == 1989));
}

#[test]
fn m1_same_label_different_climate_invalidates_cache() {
    use fieldshift_eval::adapter::IndicatorLossModel;
    use fieldshift_model::catalog::enumerate_plans;
    let cfg = model_common::config();
    let cat = model_common::catalog(&cfg);
    let soil = model_common::soil(&cfg, None, None);
    let params = model_common::params(&cfg, &cat);
    let (plans, _) = enumerate_plans(&cat, &soil, 0.20).expect("plans");
    let s = split_scenarios(&climate_from_fixture(), &EvalProtocol::default()).expect("split");
    let mut model = IndicatorLossModel::new(&plans, &params);
    model.fit(&s.construction).expect("fit");
    let mut prior = None;
    let mut changed = false;
    for k in 0..60 {
        let mut sc = s.construction[k % s.construction.len()].clone();
        sc.label = "same_label".into();
        sc.start = -1;
        let got = model.components(0, &sc).expect("cached");
        let mut fresh = IndicatorLossModel::new(&plans, &params);
        fresh.fit(&s.construction).expect("fit");
        assert_eq!(
            got,
            fresh.components(0, &sc).expect("fresh"),
            "iteration {k}"
        );
        if prior.is_some_and(|x| x != got) {
            changed = true;
        }
        prior = Some(got);
    }
    assert!(changed, "negative control needs distinct indicator values");
}

#[test]
fn weight_grid_protocol_order() {
    let g = fieldshift_eval::evaluation::weight_grid();
    assert_eq!(g.len(), 15);
    assert_eq!(g[0], [1.0, 0.0, 0.0]);
    assert_eq!(g[4], [0.5, 0.25, 0.25]);
    assert_eq!(g[14], [0.0, 0.0, 1.0]);
    for (i, w) in g.iter().enumerate() {
        assert!(!g[..i].contains(w));
        assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }
}
#[test]
fn split_boundaries_and_leap_day_removal() {
    let s = split_scenarios(&climate_from_fixture(), &EvalProtocol::default()).expect("split");
    assert_eq!((s.construction.len(), s.test.len()), (28, 14));
    assert!(s
        .construction
        .iter()
        .all(|x| x.last_date.as_str() < "2010-09-01" && x.precip.len() == 730));
    assert!(s.test.iter().all(|x| x.first_date.as_str() >= "2010-09-01"));
    let dates = fieldshift_eval::evaluation::agri_year_dates(1983);
    assert_eq!(dates.len(), 365);
    assert_eq!(dates[180], (1984, 2, 28));
    assert_eq!(dates[181], (1984, 3, 1));
}
#[test]
fn fill_values_all_four_variables_exclude_pairs() {
    for k in 0..4 {
        let mut c = climate_from_fixture();
        let i = c
            .dates
            .iter()
            .position(|d| *d == (1990, 1, 15))
            .expect("date");
        match k {
            0 => c.prectotcorr[i] = -999.0,
            1 => c.t2m[i] = -999.0,
            2 => c.t2m_max[i] = -999.0,
            _ => c.t2m_min[i] = -999.0,
        };
        let s = split_scenarios(&c, &EvalProtocol::default()).expect("split");
        assert_eq!(s.construction.len(), 26);
        assert_eq!(s.excluded.len(), 2);
        assert!(!s.construction_years.iter().any(|y| y.start == 1989));
    }
}
#[test]
fn absent_day_is_not_imputed_zero() {
    let mut c = climate_from_fixture();
    let i = c
        .dates
        .iter()
        .position(|d| *d == (2015, 3, 3))
        .expect("date");
    c.dates.remove(i);
    c.prectotcorr.remove(i);
    c.t2m.remove(i);
    c.t2m_max.remove(i);
    c.t2m_min.remove(i);
    assert_eq!(
        split_scenarios(&c, &EvalProtocol::default())
            .expect("split")
            .test
            .len(),
        12
    );
}
#[test]
fn mean_climate_refuses_missing_day() {
    let mut s = split_scenarios(&climate_from_fixture(), &EvalProtocol::default()).expect("split");
    for y in &mut s.construction_years {
        y.precip[10] = f64::NAN;
    }
    assert!(mean_climate(&s.construction_years.iter().collect::<Vec<_>>()).is_err());
}
#[test]
fn duplicate_dates_and_column_lengths_are_rejected() {
    let mut c = climate_from_fixture();
    c.dates[1] = c.dates[0];
    assert!(split_scenarios(&c, &EvalProtocol::default()).is_err());
    let mut c = climate_from_fixture();
    c.t2m.pop();
    assert!(split_scenarios(&c, &EvalProtocol::default()).is_err());
}
#[test]
fn changing_test_data_preserves_construction_and_reference() {
    let c = climate_from_fixture();
    let p = EvalProtocol::default();
    let a = split_scenarios(&c, &p).expect("split");
    let mut changed = c;
    for (i, d) in changed.dates.iter().enumerate() {
        if *d >= (2010, 9, 1) {
            changed.prectotcorr[i] *= 0.1;
            changed.t2m_max[i] += 5.0;
        }
    }
    let b = split_scenarios(&changed, &p).expect("split");
    assert_eq!(
        construction_precip_fingerprint(&a.construction),
        construction_precip_fingerprint(&b.construction)
    );
    assert_eq!(
        dry_threshold(&a.construction_years, &p),
        dry_threshold(&b.construction_years, &p)
    );
    assert_ne!(a.test[0].content_key(), b.test[0].content_key());
}
#[test]
fn manual_selection_and_tie_breaks() {
    use fieldshift_eval::evaluation::{regret, select_conv, select_exp, select_mmr};
    let ids = [vec![0], vec![1], vec![2]];
    let refs = ids.iter().collect::<Vec<_>>();
    let l = vec![
        vec![0.0, 0.0, 0.9],
        vec![0.4, 0.4, 0.4],
        vec![0.5, 0.5, 0.5],
    ];
    assert_eq!(select_mmr(&l, &refs, 9), 1);
    assert_eq!(select_exp(&l, &refs, 9), 0);
    assert_eq!(select_conv(&[0.1, 0.3, 0.3], &l, &refs, 9), 0);
    let r = regret(&l);
    assert!(r.iter().flatten().all(|x| *x >= 0.0));
    for j in 0..3 {
        assert!(r.iter().any(|row| row[j] == 0.0));
    }
    let tied = vec![
        vec![0.1, 0.3, 0.2],
        vec![0.1, 0.3, 0.2],
        vec![0.9, 0.9, 0.9],
    ];
    let reversed = [vec![9], vec![1], vec![2]];
    let refs = reversed.iter().collect::<Vec<_>>();
    assert_eq!(select_mmr(&tied, &refs, 9), 1);
    assert_eq!(select_exp(&tied, &refs, 9), 1);
    assert_eq!(select_conv(&[0.5, 0.5, 0.6], &tied, &refs, 9), 1);
}
