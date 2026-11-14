// AI-assisted (OpenBot agent)
//! Portes 1-para-1 de `tests/test_optimization.py` (46 testes Python): matriz de perdas, regret,
//! seleção lexicográfica, Pareto, alternativas, cenários, ausentes, portões, `run` sintético,
//! invariantes de ótimo, T2 (vazamento), determinismo e execução no snapshot real.

mod common;

use std::collections::BTreeSet;

use common::*;
use fieldshift_model::catalog::*;
use fieldshift_model::indicators::{loss_components, ClimatePair};
use fieldshift_model::optimization::*;
use fieldshift_model::pareto::*;
use fieldshift_model::scenarios::*;
use fieldshift_model::selection::*;
use serde_json::Value;

struct Env {
    cfg: Config,
    cat: Catalog,
    soil: Soil,
}

fn env() -> Env {
    let cfg = config();
    let cat = catalog(&cfg);
    let soil = soil(&cfg, None, None);
    Env { cfg, cat, soil }
}

fn do_run(e: &Env, clim: &ClimateFrame, opts: &RunOptions) -> RunResult {
    let prm = params(&e.cfg, &e.cat);
    run(clim, &e.cat, &e.soil, &prm, "synthetic-test", opts).expect("run")
}

fn mat(rows: &[&[f64]]) -> Matrix {
    let cols = rows[0].len();
    Matrix {
        rows: rows.len(),
        cols,
        data: rows.iter().flat_map(|r| r.iter().copied()).collect(),
    }
}

fn pid2(v: &[[i32; 2]]) -> Vec<PlanId> {
    v.iter().map(|x| [x[0], x[1], 0, 0, 0, 0]).collect()
}

fn pid1(i: i32) -> PlanId {
    [i, 0, 0, 0, 0, 0]
}

// ------------------------------------------------------------------ seleção e Pareto (1..16)
#[test]
fn test_weight_grid_is_15_unique_triples_in_quarters() {
    let g = weight_grid();
    assert_eq!(g.len(), 15);
    assert_eq!(g[0], [1.0, 0.0, 0.0]);
    assert_eq!(g[4], DEFAULT_WEIGHTS);
    assert_eq!(g[14], [0.0, 0.0, 1.0]);
    for w in &g {
        assert_eq!(w.iter().sum::<f64>(), 1.0);
        assert!(w.iter().all(|x| (x * 4.0).fract() == 0.0));
    }
    for i in 0..g.len() {
        for j in i + 1..g.len() {
            assert_ne!(g[i], g[j]);
        }
    }
}

#[test]
fn test_toy_3x3_by_hand() {
    let l = mat(&[&[1.0, 3.0, 2.0], &[2.0, 2.0, 2.0], &[3.0, 1.0, 4.0]]);
    let r = regret_matrix(&l);
    assert_eq!(r.data, vec![0.0, 2.0, 0.0, 1.0, 1.0, 0.0, 2.0, 0.0, 2.0]);
    let id = pid2(&[[0, 2], [0, 1], [0, 3]]);
    assert_eq!(select_mmr(&l, &id).index, 1);
    let exp = select_exp(&l, &id);
    assert_eq!(
        (exp.index, exp.tied_after_level.clone(), exp.id_tiebreak),
        (1, vec![2, 1], false)
    );
    let lbar = [1.5, 1.5, 3.0];
    let conv = select_conv(&l, &lbar, &id);
    assert_eq!(
        (conv.index, conv.tied_after_level.clone(), conv.id_tiebreak),
        (1, vec![2, 2], true)
    );
    assert_eq!(
        select_conv(&l, &lbar, &pid2(&[[0, 1], [0, 2], [0, 3]])).index,
        0
    );
}

#[test]
fn test_mmr_second_criterion_mean_regret() {
    let l = mat(&[&[0.0, 2.0, 2.0], &[2.0, 2.0, 0.0], &[2.0, 0.0, 1.0]]);
    let s = select_mmr(&l, &pid2(&[[0, 1], [0, 2], [0, 3]]));
    assert_eq!(
        (s.index, s.tied_after_level, s.id_tiebreak),
        (2, vec![3, 1], false)
    );
}

#[test]
fn test_numeric_tolerance_is_nine_decimals_then_id() {
    let l = mat(&[&[0.1, 0.5], &[0.1 + 1e-12, 0.5]]);
    let s = select_mmr(&l, &pid2(&[[0, 2], [0, 1]]));
    assert_eq!((s.index, s.id_tiebreak), (1, true));
    let l2 = mat(&[&[0.1, 0.5], &[0.1 + 1e-6, 0.5]]);
    assert_eq!(select_mmr(&l2, &pid2(&[[0, 2], [0, 1]])).index, 0);
}

#[test]
fn test_regret_nonnegative_and_oracle_zero() {
    let mut data = Vec::new();
    let mut x = 12345u64;
    for _ in 0..40 * 9 {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        data.push(((x >> 11) as f64) / ((1u64 << 53) as f64));
    }
    let l = Matrix {
        rows: 40,
        cols: 9,
        data,
    };
    let r = regret_matrix(&l);
    assert!(r.data.iter().all(|v| *v >= 0.0));
    for j in 0..9 {
        assert!((0..40).any(|i| r.at(i, j) == 0.0));
    }
}

fn check_selection_order_invariance(seed: u64) {
    let (n, s) = (30usize, 8usize);
    let mut x = seed.wrapping_add(7);
    let mut next = || {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (x >> 33) as usize
    };
    let data: Vec<f64> = (0..n * s).map(|_| (next() % 4) as f64 / 4.0).collect();
    let lbar: Vec<f64> = (0..n).map(|_| (next() % 4) as f64 / 4.0).collect();
    let id: Vec<PlanId> = (0..n)
        .map(|i| [(i / 6) as i32, (i % 6) as i32, 0, 0, 0, 0])
        .collect();
    let l = Matrix {
        rows: n,
        cols: s,
        data,
    };
    let base = (
        id[select_mmr(&l, &id).index],
        id[select_exp(&l, &id).index],
        id[select_conv(&l, &lbar, &id).index],
    );
    for _ in 0..6 {
        let mut pp: Vec<usize> = (0..n).collect();
        let mut ss: Vec<usize> = (0..s).collect();
        for i in (1..n).rev() {
            pp.swap(i, next() % (i + 1));
        }
        for i in (1..s).rev() {
            ss.swap(i, next() % (i + 1));
        }
        let mut d2 = Vec::new();
        for &p in &pp {
            for &c in &ss {
                d2.push(l.at(p, c));
            }
        }
        let l2 = Matrix {
            rows: n,
            cols: s,
            data: d2,
        };
        let id2: Vec<PlanId> = pp.iter().map(|&p| id[p]).collect();
        let lb2: Vec<f64> = pp.iter().map(|&p| lbar[p]).collect();
        let got = (
            id2[select_mmr(&l2, &id2).index],
            id2[select_exp(&l2, &id2).index],
            id2[select_conv(&l2, &lb2, &id2).index],
        );
        assert_eq!(got, base);
    }
}

#[test]
fn test_selection_invariant_to_plan_and_scenario_order_0() {
    check_selection_order_invariance(0);
}

#[test]
fn test_selection_invariant_to_plan_and_scenario_order_1() {
    check_selection_order_invariance(1);
}

#[test]
fn test_selection_invariant_to_plan_and_scenario_order_2() {
    check_selection_order_invariance(2);
}

#[test]
fn test_selection_invariant_to_plan_and_scenario_order_3() {
    check_selection_order_invariance(3);
}

#[test]
fn test_loss_matrix_fixed_order_and_weights() {
    let t = Tensor3 {
        n: 1,
        s: 1,
        data: vec![[0.2, 0.4, 0.6]],
    };
    assert_eq!(
        loss_matrix(&t, [0.5, 0.25, 0.25]).data[0],
        0.5 * 0.2 + 0.25 * 0.4 + 0.25 * 0.6
    );
    assert_eq!(loss_matrix(&t, [0.0, 0.0, 1.0]).data[0], 0.6);
}

fn check_pareto_bruteforce(seed: u64) {
    let mut x = seed.wrapping_add(99);
    let mut next = || {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (x >> 33) as usize
    };
    let f: Vec<[f64; 4]> = (0..60)
        .map(|_| [(); 4].map(|()| (next() % 5) as f64 / 4.0))
        .collect();
    let ids: Vec<PlanId> = (0..60).map(|i| pid1(i * 37 % 60)).collect();
    let (front, _) = pareto_front(&f, &ids);
    let naive: Vec<usize> = (0..60)
        .filter(|&i| {
            !(0..60)
                .any(|j| (0..4).all(|k| f[j][k] <= f[i][k]) && (0..4).any(|k| f[j][k] < f[i][k]))
        })
        .collect();
    let key = |v: &[f64; 4]| {
        v.map(|val| {
            if val == 0.0 {
                0.0f64.to_bits()
            } else {
                val.to_bits()
            }
        })
    };
    let fv: BTreeSet<[u64; 4]> = front.iter().map(|&i| key(&f[i])).collect();
    let nv: BTreeSet<[u64; 4]> = naive.iter().map(|&i| key(&f[i])).collect();
    assert_eq!(fv, nv);
    for &r in &front {
        let same: Vec<usize> = (0..60).filter(|&i| key(&f[i]) == key(&f[r])).collect();
        assert_eq!(r, *same.iter().min_by_key(|&&i| ids[i]).unwrap());
    }
}

#[test]
fn test_pareto_matches_bruteforce_with_duplicates_0() {
    check_pareto_bruteforce(0);
}

#[test]
fn test_pareto_matches_bruteforce_with_duplicates_1() {
    check_pareto_bruteforce(1);
}

#[test]
fn test_pareto_matches_bruteforce_with_duplicates_2() {
    check_pareto_bruteforce(2);
}

#[test]
fn test_pareto_matches_bruteforce_with_duplicates_3() {
    check_pareto_bruteforce(3);
}

#[test]
fn test_pareto_matches_bruteforce_with_duplicates_4() {
    check_pareto_bruteforce(4);
}

#[test]
fn test_pareto_strict_dominance_example() {
    let f = [
        [1.0, 1.0, 0.0, 0.0],
        [1.0, 2.0, 0.0, 0.0],
        [2.0, 1.0, 0.0, 0.0],
        [1.0, 1.0, 0.0, 0.0],
        [0.0, 3.0, 0.0, 0.0],
        [3.0, 0.0, 0.0, 0.0],
    ];
    let ids: Vec<PlanId> = (0..6).map(pid1).collect();
    let (front, eq) = pareto_front(&f, &ids);
    assert_eq!(front, vec![0, 4, 5]);
    assert_eq!(eq[&0], vec![3]);
}

// ------------------------------------------------------------------ cenários e split (17..28)
#[test]
fn test_scenarios_split_and_labels() {
    let clim = synth_climate(0);
    let c = build_scenarios(&clim, Split::Construction).unwrap();
    let t = build_scenarios(&clim, Split::Test).unwrap();
    assert_eq!((c.scenarios.len(), c.n_valid(), c.n_expected), (28, 28, 28));
    assert_eq!((t.scenarios.len(), t.n_valid(), t.n_expected), (14, 14, 14));
    assert_eq!(c.scenarios[0].label, "1981/82+1982/83");
    assert_eq!(c.scenarios[27].label, "2008/09+2009/10");
    assert_eq!(t.scenarios[0].label, "2010/11+2011/12");
    assert_eq!(t.scenarios[13].label, "2023/24+2024/25");
    assert!(c.scenarios.iter().map(|s| s.y).max().unwrap() + 1 < TEST_FIRST_Y);
    assert!(t.scenarios.iter().map(|s| s.y).min().unwrap() >= TEST_FIRST_Y);
    assert!(c
        .valid()
        .iter()
        .chain(t.valid().iter())
        .all(|s| s.pair.as_ref().unwrap().p.len() == 730));
    assert_eq!(c.ag_years.keys().max(), Some(&2009));
}

#[test]
fn test_agri_year_alignment_and_feb29_dropped() {
    let mut clim = synth_climate(0);
    assert!(clim.set((1984, 2, 29), 0, 12345.0));
    let c = build_scenarios(&clim, Split::Construction).unwrap();
    let s0 = c.scenarios[0].pair.as_ref().unwrap();
    let at = |d: (i32, u8, u8)| clim.get(d).unwrap()[0];
    assert_eq!(s0.p[0], at((1981, 9, 1)));
    assert_eq!(s0.p[122], at((1982, 1, 1)));
    assert_eq!(s0.p[364], at((1982, 8, 31)));
    assert_eq!(s0.p[365], at((1982, 9, 1)));
    assert_eq!(s0.p[729], at((1983, 8, 31)));
    assert!(c
        .valid()
        .iter()
        .all(|s| !s.pair.as_ref().unwrap().p.contains(&12345.0)));
    let y1983 = &c.ag_years[&1983];
    assert_eq!(y1983.p.len(), 365);
    assert_eq!(y1983.p[md_to_t(2, 28).unwrap() as usize], at((1984, 2, 28)));
    assert_eq!(y1983.p[md_to_t(3, 1).unwrap() as usize], at((1984, 3, 1)));
}

#[test]
fn test_test_period_cannot_leak_into_construction() {
    let clim = synth_climate(0);
    let c = build_scenarios(&clim, Split::Construction).unwrap();
    let at = |d: (i32, u8, u8)| clim.get(d).unwrap()[0];
    let last = c.scenarios[27].pair.as_ref().unwrap();
    assert_eq!(last.p[729], at((2010, 8, 31)));
    assert_eq!(c.ag_years.keys().max(), Some(&2009));
}

fn check_missing_variable(var: usize, name: &str) {
    let mut clim = synth_climate(0);
    clim.set((1995, 3, 10), var, f64::NAN);
    let c = build_scenarios(&clim, Split::Construction).unwrap();
    let bad: Vec<&str> = c
        .scenarios
        .iter()
        .filter(|s| !s.valid)
        .map(|s| s.label.as_str())
        .collect();
    assert_eq!(bad, ["1993/94+1994/95", "1994/95+1995/96"]);
    assert_eq!(c.n_valid(), 26);
    let log = c.missing_log();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0]["ag_year"], 1994);
    assert_eq!(log[0][name], 1);
    assert!(c
        .excluded()
        .iter()
        .all(|e| e["reason"].as_str().unwrap().starts_with("dados ausentes")));
}

#[test]
fn test_missing_in_any_variable_excludes_scenarios_for_everyone_prectotcorr() {
    check_missing_variable(0, "PRECTOTCORR");
}

#[test]
fn test_missing_in_any_variable_excludes_scenarios_for_everyone_t2m() {
    check_missing_variable(1, "T2M");
}

#[test]
fn test_missing_in_any_variable_excludes_scenarios_for_everyone_t2m_max() {
    check_missing_variable(2, "T2M_MAX");
}

#[test]
fn test_missing_in_any_variable_excludes_scenarios_for_everyone_t2m_min() {
    check_missing_variable(3, "T2M_MIN");
}

#[test]
fn test_missing_date_row_counts_as_missing() {
    let mut clim = synth_climate(0);
    assert!(clim.remove((1990, 7, 4)));
    let c = build_scenarios(&clim, Split::Construction).unwrap();
    assert_eq!(c.n_valid(), 26);
    assert_eq!(c.missing_log()[0]["ag_year"], 1989);
}

#[test]
fn test_999_must_be_nan_before_scenarios() {
    let mut clim = synth_climate(0);
    // Na fronteira de entrada (ClimateFrame), sentinelas convertidos a NaN excluem os pares afetados,
    // e a mensagem de exclusão reproduz o dict Python {'1994:T2M_MAX': 1}.
    clim.set((1995, 3, 10), 2, f64::NAN);
    let c = build_scenarios(&clim, Split::Construction).unwrap();
    let r = &c.scenarios.iter().find(|s| !s.valid).unwrap().reason;
    assert_eq!(r, "dados ausentes {'1994:T2M_MAX': 1}");
}

#[test]
fn test_sbar_and_reference_exclude_invalid_years() {
    let mut clim = synth_climate(0);
    clim.set((1995, 3, 10), 2, f64::NAN);
    let c = build_scenarios(&clim, Split::Construction).unwrap();
    let (reference, sbar) = construction_climatology(&c).unwrap();
    assert!(!reference.labels.contains(&1994) && reference.n_years() == 28);
    let good: Vec<i32> = (1981..2010).filter(|y| *y != 1994).collect();
    let manual: f64 = good.iter().map(|y| c.ag_years[y].p[40]).sum::<f64>() / good.len() as f64;
    assert!((sbar.p[40] - manual).abs() < 1e-12 && sbar.p[40 + 365] == sbar.p[40]);
    assert!(construction_climatology(&build_scenarios(&clim, Split::Test).unwrap()).is_err());
}

#[test]
fn test_construction_gate_gives_dados_insuficientes() {
    let e = env();
    let mut clim = synth_climate(0);
    for y in (1983..2010).step_by(3) {
        clim.set((y, 11, 15), 0, f64::NAN);
    }
    for y in (1984..2010).step_by(3) {
        clim.set((y, 11, 15), 0, f64::NAN);
    }
    let r = do_run(&e, &clim, &RunOptions::default());
    assert_eq!(r.status(), "DADOS_INSUFICIENTES");
    assert!(r
        .get("message")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("dados insuficientes"));
    assert!(r.get("policies").unwrap().as_object().unwrap().is_empty());
}

#[test]
fn test_test_gate_gives_dados_insuficientes() {
    let e = env();
    let mut clim = synth_climate(0);
    for y in (2011..2025).step_by(2) {
        clim.set((y, 11, 15), 3, f64::NAN);
    }
    let r = do_run(&e, &clim, &RunOptions::default());
    assert_eq!(r.status(), "DADOS_INSUFICIENTES");
    assert!(
        r.get("scenarios").unwrap()["test"]["n_valid"]
            .as_u64()
            .unwrap()
            < 10
    );
}

// ------------------------------------------------------------------ run sintético (29..42)
#[test]
fn test_run_basic_structure() {
    let e = env();
    let r = do_run(&e, &synth_climate(0), &RunOptions::default());
    assert_eq!(
        (r.status(), r.get("n_plans").unwrap().as_u64().unwrap()),
        ("OK", 576)
    );
    assert_eq!(r.get("risk_limit_pct").unwrap(), 20);
    assert_eq!(r.get("snapshot_id").unwrap(), "synthetic-test");
    assert_eq!(r.get("config_hash").unwrap().as_str().unwrap().len(), 64);
    let (plans, _) = enumerate_plans(&e.cat, &e.soil, 0.20).unwrap();
    for n in ["P0", "P1", "P1b", "P2"] {
        let id = r.policy_plan_id(n).unwrap();
        assert!(
            verify_plan(&e.cat, &e.soil, find_plan(&plans, &id).unwrap(), 0.20)
                .unwrap()
                .is_empty()
        );
    }
    let sens = r.get("sensitivity").unwrap().as_array().unwrap();
    assert_eq!(sens.len(), 15);
    for (row, w) in sens.iter().zip(weight_grid()) {
        let got: Vec<f64> = row["weights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect();
        assert_eq!(got, w.to_vec());
    }
    let std_row = &sens[4];
    for n in ["P1", "P1b", "P2"] {
        assert_eq!(std_row[n], r.get("policies").unwrap()[n]["plan"]["id"]);
    }
}

fn metric(r: &RunResult, p: &str, k: &str) -> f64 {
    r.get("policies").unwrap()[p]["metrics_construction"][k]
        .as_f64()
        .unwrap()
}

#[test]
fn test_policies_satisfy_optimality_invariants() {
    let e = env();
    let r = do_run(&e, &synth_climate(0), &RunOptions::default());
    for n in ["P0", "P1", "P1b"] {
        assert!(metric(&r, "P2", "max_regret") <= metric(&r, n, "max_regret") + 1e-12);
    }
    for n in ["P0", "P1", "P2"] {
        assert!(metric(&r, "P1b", "mean_L") <= metric(&r, n, "mean_L") + 1e-12);
    }
    for n in ["P0", "P1b", "P2"] {
        assert!(metric(&r, "P1", "L_sbar") <= metric(&r, n, "L_sbar") + 1e-12);
    }
    assert!(["P0", "P1", "P1b", "P2"]
        .iter()
        .all(|n| metric(&r, n, "max_regret") >= 0.0));
}

#[test]
fn test_run_matches_independent_bruteforce() {
    let e = env();
    let clim = synth_climate(0);
    let r = do_run(&e, &clim, &RunOptions::default());
    let (plans, _) = enumerate_plans(&e.cat, &e.soil, 0.20).unwrap();
    let cset = build_scenarios(&clim, Split::Construction).unwrap();
    let (reference, sbar) = construction_climatology(&cset).unwrap();
    let prm = params(&e.cfg, &e.cat).with_reference(reference);
    let w = DEFAULT_WEIGHTS;
    let ls = |g: [f64; 3]| w[0] * g[0] + w[1] * g[1] + w[2] * g[2];
    let pairs: Vec<&ClimatePair> = cset
        .valid()
        .iter()
        .map(|s| s.pair.as_ref().unwrap())
        .collect();
    let l: Vec<Vec<f64>> = plans
        .iter()
        .map(|p| {
            pairs
                .iter()
                .map(|pr| ls(loss_components(p, pr, &prm).unwrap()))
                .collect()
        })
        .collect();
    let n = pairs.len();
    let oracle: Vec<f64> = (0..n)
        .map(|j| l.iter().map(|row| row[j]).fold(f64::INFINITY, f64::min))
        .collect();
    let r6 = |v: f64| (v * 1e9).round_ties_even() / 1e9;
    let max_r = |i: usize| {
        (0..n)
            .map(|j| l[i][j] - oracle[j])
            .fold(f64::NEG_INFINITY, f64::max)
    };
    let mean_r = |i: usize| (0..n).map(|j| l[i][j] - oracle[j]).sum::<f64>() / n as f64;
    let mean_l = |i: usize| l[i].iter().sum::<f64>() / n as f64;
    let pick = |key: &dyn Fn(usize) -> (f64, f64, PlanId)| {
        (0..plans.len())
            .min_by(|&a, &b| {
                let (ka, kb) = (key(a), key(b));
                ka.0.partial_cmp(&kb.0)
                    .unwrap()
                    .then(ka.1.partial_cmp(&kb.1).unwrap())
                    .then(ka.2.cmp(&kb.2))
            })
            .unwrap()
    };
    let mmr = pick(&|i| (r6(max_r(i)), r6(mean_r(i)), plans[i].id));
    let exp = pick(&|i| (r6(mean_l(i)), r6(max_r(i)), plans[i].id));
    let lb: Vec<f64> = plans
        .iter()
        .map(|p| ls(loss_components(p, &sbar, &prm).unwrap()))
        .collect();
    let conv = pick(&|i| (r6(lb[i]), r6(mean_l(i)), plans[i].id));
    assert_eq!(r.policy_plan_id("P2"), Some(plans[mmr].id));
    assert_eq!(r.policy_plan_id("P1b"), Some(plans[exp].id));
    assert_eq!(r.policy_plan_id("P1"), Some(plans[conv].id));
}

#[test]
fn test_run_pareto_consistent_with_objectives() {
    let e = env();
    let r = do_run(&e, &synth_climate(0), &RunOptions::default());
    let rows = r.get("objectives").unwrap().as_array().unwrap();
    let f: Vec<[f64; 4]> = rows
        .iter()
        .map(|row| {
            let a = row.as_array().unwrap();
            [
                a[1].as_f64().unwrap(),
                a[2].as_f64().unwrap(),
                a[3].as_f64().unwrap(),
                a[4].as_f64().unwrap(),
            ]
        })
        .collect();
    let naive: Vec<usize> = (0..f.len())
        .filter(|&i| {
            !(0..f.len())
                .any(|j| (0..4).all(|k| f[j][k] <= f[i][k]) && (0..4).any(|k| f[j][k] < f[i][k]))
        })
        .collect();
    let front_ids: Vec<Value> = r.get("pareto").unwrap()["front_ids"]
        .as_array()
        .unwrap()
        .clone();
    let (plans, _) = enumerate_plans(&e.cat, &e.soil, 0.20).unwrap();
    for id in &front_ids {
        let pid: PlanId = ids_of(id).try_into().unwrap();
        let i = plan_index(&plans, &pid).unwrap();
        assert!(
            naive.iter().any(|&j| f[j] == f[i]),
            "{pid:?} não é não dominado"
        );
    }
    assert_eq!(
        front_ids.len() as u64,
        r.get("pareto").unwrap()["n_front"].as_u64().unwrap()
    );
}

#[test]
fn test_alternatives_are_distinct() {
    let e = env();
    let r = do_run(&e, &synth_climate(0), &RunOptions::default());
    let (plans, _) = enumerate_plans(&e.cat, &e.soil, 0.20).unwrap();
    let alts = r.get("alternatives").unwrap().as_array().unwrap();
    let sigs: Vec<[usize; 4]> = alts
        .iter()
        .map(|a| {
            find_plan(&plans, &ids_of(&a["plan_id"]).try_into().unwrap())
                .unwrap()
                .signature()
        })
        .collect();
    assert!(sigs.len() <= K_ALTERNATIVES);
    for i in 0..sigs.len() {
        for j in 0..i {
            assert_ne!(sigs[i], sigs[j]);
            if alts[i]["reason"] != "policy" && alts[i]["hamming_relaxed"] == false {
                assert!(hamming(&sigs[i], &sigs[j]) >= MIN_HAMMING);
            }
        }
    }
    let mut labels: Vec<String> = alts
        .iter()
        .flat_map(|a| {
            a["labels"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().to_string())
        })
        .collect();
    labels.sort();
    assert_eq!(labels, ["P0", "P1", "P1b", "P2"]);
}

#[test]
fn test_alternatives_never_pad_with_duplicates() {
    let mk = |sig: [usize; 4], i: i32| {
        let occ = |w: usize, c: usize| Occupation {
            window: w,
            crop: c,
            sow_t: None,
            emerge_t: None,
            harvest_t: None,
            dec: None,
        };
        Plan {
            id: pid1(i),
            occupations: [
                occ(0, sig[0]),
                occ(1, sig[1]),
                occ(2, sig[2]),
                occ(3, sig[3]),
            ],
        }
    };
    let plans = vec![
        mk([0, 2, 0, 2], 0),
        mk([0, 2, 0, 2], 1),
        mk([1, 2, 1, 2], 2),
    ];
    let f = [[0.1; 4], [0.1; 4], [0.2; 4]];
    let ids: Vec<PlanId> = plans.iter().map(|p| p.id).collect();
    let (front, _) = pareto_front(&f, &ids);
    let alts = select_alternatives(
        &plans,
        &f,
        &front,
        &[("P1".into(), 0)],
        [0.5, 0.25, 0.25],
        6,
        2,
    );
    assert_eq!(alts.len(), 1);
    assert_eq!(alts[0].labels, vec!["P1".to_string()]);
}

#[test]
fn test_fallow_reporting() {
    let e = env();
    let r = do_run(&e, &synth_climate(0), &RunOptions::default());
    let f = r.get("fallow").unwrap();
    assert_eq!(
        (
            f["total"].as_i64(),
            f["com_pousio"].as_i64(),
            f["pousio_J2"].as_i64(),
            f["pousio_J4"].as_i64(),
            f["pousio_J2_e_J4"].as_i64()
        ),
        (Some(576), Some(176), Some(96), Some(96), Some(16))
    );
    let u = r.get("fallow_usage").unwrap();
    assert_eq!(
        (u["x_plans_with_fallow"].as_i64(), u["grid_size"].as_i64()),
        (Some(176), Some(15))
    );
    assert_eq!(u["front_size"], r.get("pareto").unwrap()["n_front"]);
}

#[test]
fn test_run_is_deterministic_and_json_roundtrip() {
    let e = env();
    let clim = synth_climate(0);
    let a = do_run(&e, &clim, &RunOptions::default());
    let b = do_run(&e, &clim, &RunOptions::default());
    assert_eq!(a.result_hash(), b.result_hash());
    assert_eq!(a.to_value(false), b.to_value(false));
    let s = serde_json::to_string(&a.to_value(false)).unwrap();
    let parsed: Value = serde_json::from_str(&s).unwrap();
    assert_eq!(parsed, a.to_value(false));
}

#[test]
fn test_meta_is_excluded_from_result_hash() {
    let e = env();
    let clim = synth_climate(0);
    let mut b = do_run(&e, &clim, &RunOptions::default());
    let h = b.result_hash();
    b.meta
        .insert("wall_seconds".into(), serde_json::json!(1.23));
    assert_eq!(b.result_hash(), h);
    assert!(b.to_value(true).get("meta").is_some() && b.to_value(false).get("meta").is_none());
}

#[test]
fn test_t2_test_years_never_enter_the_choice() {
    let e = env();
    let clim = synth_climate(0);
    let base = do_run(&e, &clim, &RunOptions::default());
    let mut pert = ClimateFrame::new();
    let test_days: Vec<(i32, u8, u8)> = (2010..=2025)
        .flat_map(|y| (1..=12u8).flat_map(move |m| (1..=month_days(y, m)).map(move |d| (y, m, d))))
        .filter(|d| *d >= (2010, 9, 1))
        .collect();
    let mut shuffled: Vec<f64> = test_days
        .iter()
        .map(|d| clim.get(*d).unwrap()[0] * 3.0)
        .collect();
    shuffled.rotate_left(777);
    let mut idx = 0;
    for y in 1981..=2025 {
        for m in 1..=12u8 {
            for d in 1..=month_days(y, m) {
                let mut v = *clim.get((y, m, d)).unwrap();
                if (y, m, d) >= (2010, 9, 1) {
                    v[0] = shuffled[idx];
                    v[2] += 6.0;
                    idx += 1;
                }
                pert.insert(y, m, d, v).unwrap();
            }
        }
    }
    let res = do_run(&e, &pert, &RunOptions::default());
    for k in [
        "policies",
        "sensitivity",
        "objectives",
        "pareto",
        "alternatives",
        "climatology",
        "fallow_usage",
    ] {
        assert_eq!(res.get(k), base.get(k), "{k}");
    }
    assert_eq!(res.result_hash(), base.result_hash());
    let mut pc = ClimateFrame::new();
    for y in 1981..=2025 {
        for m in 1..=12u8 {
            for d in 1..=month_days(y, m) {
                let mut v = *clim.get((y, m, d)).unwrap();
                if (y, m, d) < (2010, 9, 1) {
                    v[0] *= 0.2;
                }
                pc.insert(y, m, d, v).unwrap();
            }
        }
    }
    assert_ne!(
        do_run(&e, &pc, &RunOptions::default()).result_hash(),
        base.result_hash()
    );
}

#[test]
fn test_inviavel_result_and_no_relaxation() {
    let mut e = env();
    for row in e.cat.zarc.values_mut() {
        for v in row.iter_mut() {
            if *v == 20 {
                *v = 30;
            }
        }
    }
    let r = do_run(&e, &synth_climate(0), &RunOptions::default());
    assert_eq!(
        (r.status(), r.get("risk_limit_pct").unwrap()),
        ("INVIAVEL", &Value::from(20))
    );
    assert_eq!(r.get("diagnosis").unwrap()["status"], "INVIAVEL");
    assert!(!r
        .get("rejections_summary")
        .unwrap()
        .as_object()
        .unwrap()
        .is_empty());
    assert!(r.get("policies").unwrap().as_object().unwrap().is_empty());
    assert_eq!(r.get("n_plans").unwrap(), 0);
}

#[test]
fn test_p0_outside_zarc_leaves_regret_comparison() {
    let mut e = env();
    e.soil = soil(&e.cfg, Some("AD1"), None);
    let r = do_run(&e, &synth_climate(0), &RunOptions::default());
    assert_eq!(
        (r.status(), r.get("n_plans").unwrap().as_u64().unwrap()),
        ("OK", 169)
    );
    let p0 = &r.get("policies").unwrap()["P0"];
    assert_eq!(p0["status"], "INVIAVEL_SOB_ZARC");
    assert!(p0["message"]
        .as_str()
        .unwrap()
        .contains("inviável sob ZARC"));
    assert!(p0.get("plan").is_none() && !p0["reasons"].as_array().unwrap().is_empty());
    for n in ["P1", "P1b", "P2"] {
        assert_eq!(r.get("policies").unwrap()[n]["status"], "OK");
    }
    let alts = r.get("alternatives").unwrap().as_array().unwrap();
    assert!(alts
        .iter()
        .all(|a| !a["labels"].as_array().unwrap().iter().any(|l| l == "P0")));
    assert!(r.get("fallow_usage").unwrap()["policies_w_star"]
        .get("P0")
        .is_none());
}

#[test]
fn test_config_hash_tracks_configuration() {
    let e = env();
    let clim = synth_climate(0);
    let a = do_run(&e, &clim, &RunOptions::default());
    let b = do_run(
        &e,
        &clim,
        &RunOptions {
            weights: Some([0.25, 0.5, 0.25]),
            ..RunOptions::default()
        },
    );
    let mut e5 = env();
    e5.soil = soil(&e5.cfg, Some("AD5"), None);
    let c = do_run(&e5, &clim, &RunOptions::default());
    let hs = [
        a.get("config_hash").unwrap(),
        b.get("config_hash").unwrap(),
        c.get("config_hash").unwrap(),
    ];
    assert!(hs[0] != hs[1] && hs[1] != hs[2] && hs[0] != hs[2]);
    let prm = params(&e.cfg, &e.cat);
    let other = run(
        &clim,
        &e.cat,
        &e.soil,
        &prm,
        "other",
        &RunOptions::default(),
    )
    .unwrap();
    assert_eq!(other.get("config_hash"), a.get("config_hash"));
    assert_ne!(other.get("snapshot_id"), a.get("snapshot_id"));
}

#[test]
fn test_run_requires_expected_columns() {
    // Em Rust, ClimateFrame exige exatamente 4 variáveis (PRECTOTCORR, T2M, T2M_MAX, T2M_MIN) sem
    // datas duplicadas, e run valida pesos que somam 1 e risco <= 0.20.
    let mut f = ClimateFrame::new();
    f.insert(2000, 1, 1, [1.0; 4]).unwrap();
    assert!(f.insert(2000, 1, 1, [2.0; 4]).is_err());
    let e = env();
    let clim = synth_climate(0);
    let prm = params(&e.cfg, &e.cat);
    let bad = run(
        &clim,
        &e.cat,
        &e.soil,
        &prm,
        "s",
        &RunOptions {
            weights: Some([0.5, 0.5, 0.5]),
            ..RunOptions::default()
        },
    );
    assert!(bad.is_err());
}

// ------------------------------------------------------------------ snapshot real (43..46)
const GOLDEN_SHA256: &str = "b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66";

fn real_snapshot_id() -> String {
    fixture("golden_run_result.json")["snapshot_id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn test_real_snapshot_run() {
    let e = env();
    let prm = params(&e.cfg, &e.cat);
    let sid = real_snapshot_id();
    let r = run(
        &climate(),
        &e.cat,
        &e.soil,
        &prm,
        &sid,
        &RunOptions::default(),
    )
    .unwrap();
    assert_eq!(r.status(), "OK");
    assert_eq!(r.get("n_plans").unwrap(), 576);
    assert_eq!(r.get("scenarios").unwrap()["construction"]["n_valid"], 28);
    assert_eq!(r.get("scenarios").unwrap()["test"]["n_valid"], 14);
    assert_eq!(r.get("pareto").unwrap()["n_front"], 36);
    let n_eq: usize = r.get("pareto").unwrap()["equivalents"]
        .as_object()
        .unwrap()
        .values()
        .map(|v| v.as_array().unwrap().len())
        .sum();
    assert_eq!(
        r.get("pareto").unwrap()["n_front"].as_u64().unwrap() as usize + n_eq,
        50
    );
    assert_eq!(r.get("alternatives").unwrap().as_array().unwrap().len(), 6);
    assert_eq!(r.result_hash(), GOLDEN_SHA256);
}

#[test]
fn test_real_snapshot_is_deterministic() {
    let e = env();
    let prm = params(&e.cfg, &e.cat);
    let clim = climate();
    let sid = real_snapshot_id();
    let r1 = run(&clim, &e.cat, &e.soil, &prm, &sid, &RunOptions::default()).unwrap();
    let r2 = run(&clim, &e.cat, &e.soil, &prm, &sid, &RunOptions::default()).unwrap();
    assert_eq!(r1.result_hash(), r2.result_hash());
    assert_eq!(r1.to_value(false), r2.to_value(false));
}

#[test]
fn test_real_snapshot_optimality_invariants() {
    let e = env();
    let prm = params(&e.cfg, &e.cat);
    let sid = real_snapshot_id();
    let r = run(
        &climate(),
        &e.cat,
        &e.soil,
        &prm,
        &sid,
        &RunOptions::default(),
    )
    .unwrap();
    for n in ["P0", "P1", "P1b"] {
        assert!(metric(&r, "P2", "max_regret") <= metric(&r, n, "max_regret") + 1e-12);
    }
    for n in ["P0", "P1", "P2"] {
        assert!(metric(&r, "P1b", "mean_L") <= metric(&r, n, "mean_L") + 1e-12);
    }
    for n in ["P0", "P1b", "P2"] {
        assert!(metric(&r, "P1", "L_sbar") <= metric(&r, n, "L_sbar") + 1e-12);
    }
}

#[test]
fn test_real_snapshot_t2_invariance() {
    let e = env();
    let prm = params(&e.cfg, &e.cat);
    let clim = climate();
    let sid = real_snapshot_id();
    let base = run(&clim, &e.cat, &e.soil, &prm, &sid, &RunOptions::default()).unwrap();
    let mut pert = ClimateFrame::new();
    for y in 1981..=2025 {
        for m in 1..=12u8 {
            for d in 1..=month_days(y, m) {
                if let Some(mut v) = clim.get((y, m, d)).copied() {
                    if (y, m, d) >= (2010, 9, 1) {
                        v[0] *= 2.5;
                        v[2] += 4.0;
                    }
                    pert.insert(y, m, d, v).unwrap();
                }
            }
        }
    }
    let res = run(&pert, &e.cat, &e.soil, &prm, &sid, &RunOptions::default()).unwrap();
    assert_eq!(res.result_hash(), base.result_hash());
}
