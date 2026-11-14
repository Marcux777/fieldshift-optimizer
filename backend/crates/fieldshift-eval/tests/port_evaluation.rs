// AI-assisted (OpenBot agent)
//! Behavioral ports of evaluation gates; synthetic losses are test data only.
use fieldshift_eval::evaluation::*;
use std::collections::BTreeMap;
struct Toy;
impl LossModel for Toy {
    fn fit(&mut self, s: &[Scenario]) -> Result<String, EvalError> {
        Ok(construction_precip_fingerprint(s))
    }
    fn components(&mut self, p: usize, s: &Scenario) -> Result<[f64; 3], EvalError> {
        Ok([
            ((p + s.start.unsigned_abs() as usize) % 5) as f64 / 5.0,
            p as f64 / 3.0,
            0.2,
        ])
    }
}
fn climate() -> ClimateDaily {
    let dates = (1981..2025).flat_map(agri_year_dates).collect::<Vec<_>>();
    let n = dates.len();
    ClimateDaily {
        dates,
        prectotcorr: vec![5.0; n],
        t2m: vec![24.0; n],
        t2m_max: vec![30.0; n],
        t2m_min: vec![18.0; n],
    }
}
fn plans() -> Vec<PlanInfo> {
    (0..3)
        .map(|i| PlanInfo {
            id: vec![i],
            genes: vec![(i, Some(0))],
        })
        .collect()
}
fn ids() -> BTreeMap<String, Option<Vec<i64>>> {
    POLICIES
        .iter()
        .map(|s| (s.to_string(), Some(vec![0])))
        .collect()
}
fn eval(
    plans: &[PlanInfo],
    c: &ClimateDaily,
    ids: BTreeMap<String, Option<Vec<i64>>>,
) -> Result<EvalReport, EvalError> {
    let p = EvalProtocol {
        n_boot: 50,
        ..EvalProtocol::default()
    };
    evaluate_plans(
        &EvalInput {
            plans,
            policy_ids: ids,
            p0_status: None,
            climate: c,
            weights: None,
            check_plan: None,
            run_grid: false,
            optimizer_params_hash: None,
        },
        &mut Toy,
        &p,
    )
}
#[test]
fn infeasible_report_has_outcome_first() {
    let r = eval(&[], &climate(), ids()).expect("report");
    assert_eq!(r.outcome, "INVIAVEL");
    assert!(r.to_markdown().starts_with("**DESFECHO: INVIAVEL**"));
}
#[test]
fn insufficient_report_has_outcome_first() {
    let mut c = climate();
    c.prectotcorr.fill(f64::NAN);
    let r = eval(&plans(), &c, ids()).expect("report");
    assert_eq!(r.outcome, "DADOS_INSUFICIENTES");
    assert!(r
        .to_markdown()
        .starts_with("**DESFECHO: DADOS_INSUFICIENTES**"));
}
#[test]
fn m0_rejects_outside_catalog() {
    let mut i = ids();
    i.insert("P2".into(), Some(vec![99]));
    assert!(eval(&plans(), &climate(), i)
        .expect_err("M0")
        .0
        .contains("M0"));
}
#[test]
fn missing_policy_is_error() {
    let mut i = ids();
    i.remove("P1");
    assert!(eval(&plans(), &climate(), i)
        .expect_err("policy")
        .0
        .contains("P1"));
}
#[test]
fn duplicate_plan_ids_are_error() {
    let mut p = plans();
    p[1].id = p[0].id.clone();
    assert!(eval(&p, &climate(), ids())
        .expect_err("duplicate")
        .0
        .contains("duplicados"));
}
#[test]
fn infeasible_p0_is_excluded_not_promoted() {
    let mut i = ids();
    i.insert("P0".into(), Some(vec![99]));
    let r = eval(&plans(), &climate(), i).expect("report");
    assert_eq!(r.p0_status, "inviavel_sob_ZARC_20");
    let p = r.policies.iter().find(|p| p.policy == "P0").expect("P0");
    assert!(!p.feasible);
    assert!(p.m1_max_regret.is_none());
}
#[test]
fn repeated_evaluation_is_deterministic() {
    assert_eq!(
        eval(&plans(), &climate(), ids()).expect("a").result_hash(),
        eval(&plans(), &climate(), ids()).expect("b").result_hash()
    );
}
#[test]
fn every_policy_measured_on_same_test_pairs() {
    let r = eval(&plans(), &climate(), ids()).expect("report");
    assert_eq!(r.m0_feasibility, Some(1.0));
    assert!(r.policies.iter().all(|p| p.per_scenario_regret.len() == 14));
    assert_eq!(
        r.comparisons
            .iter()
            .map(|c| c.b.as_str())
            .collect::<Vec<_>>(),
        ["P1b", "P1"]
    );
}
#[test]
fn wrong_optimizer_choice_is_flagged() {
    let ps = plans();
    let c = climate();
    let r = eval(&ps, &c, ids()).expect("report");
    let reference = r.audit_get("reference_selection").expect("reference")["P2"]
        .as_array()
        .expect("id")[0]
        .as_i64()
        .expect("id");
    let mut i = ids();
    i.insert("P2".into(), Some(vec![(reference + 1) % 3]));
    let r = eval(&ps, &c, i).expect("report");
    assert_eq!(
        r.audit_get("optimizer_matches_reference").expect("audit")["P2"],
        false
    );
}
