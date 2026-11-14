// AI-assisted (OpenBot agent)
//! Portes 1-para-1 de `tests/test_indicators.py` (25 testes Python): valores calculados à mão,
//! avaliador ingênuo independente (laços), limites [0,1], pureza e erros explícitos.

mod common;

use common::params as load_params;
use common::*;
use fieldshift_model::catalog::*;
use fieldshift_model::indicators::*;

const T: usize = 730;

struct Env {
    cat: Catalog,
    plans: Vec<Plan>,
    cfg: Config,
}

fn env() -> Env {
    let cfg = config();
    let cat = catalog(&cfg);
    let (plans, _) = enumerate_plans(&cat, &soil(&cfg, None, None), 0.20).unwrap();
    Env { cat, plans, cfg }
}

fn params(e: &Env, ref_value: f64) -> IndicatorParams {
    let r =
        ConstructionClimatology::new(&vec![vec![ref_value; 365]; 5], vec![1, 2, 3, 4, 5]).unwrap();
    load_params(&e.cfg, &e.cat).with_reference(r)
}

fn arrs(p: f64, tx: f64, tn: f64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    (vec![p; T], vec![tx; T], vec![tn; T])
}

fn pair(a: (Vec<f64>, Vec<f64>, Vec<f64>)) -> ClimatePair {
    ClimatePair::new(a.0, a.1, a.2, "t").unwrap()
}

fn find<'a>(e: &'a Env, crops: [&str; 4], decs: Option<(i32, i32)>) -> &'a Plan {
    e.plans
        .iter()
        .find(|p| {
            let keys: Vec<&str> = p
                .occupations
                .iter()
                .map(|o| e.cat.crops[o.crop].key.as_str())
                .collect();
            keys == crops
                && decs.is_none_or(|d| {
                    (p.occupations[0].dec.unwrap(), p.occupations[2].dec.unwrap()) == d
                })
        })
        .expect("plano")
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-12
}

#[test]
fn test_soja_occurrence_hand_computed() {
    let e = env();
    let prm = params(&e, 2.0);
    let (mut p, mut tx, mut tn) = arrs(2.0, 30.0, 15.0);
    let soy = e.cat.index_of("soja").unwrap();
    p[100..112].fill(0.0);
    for t in [100, 101, 102] {
        tx[t] = 41.0;
    }
    for t in [50, 51, 52, 53, 54] {
        tn[t] = 3.0;
    }
    for t in [60, 61] {
        tn[t] = 3.01;
    }
    p[137..147].fill(0.0);
    p[137..143].fill(1.0);
    let r = occurrence_indicators(&pair((p, tx, tn)), &prm, soy, 40).unwrap();
    assert_eq!(r.e1, 0.0);
    assert!(close(r.e2, 12.0 / 30.0));
    assert!(close(r.e3, 1.0 - 36.0 / 60.0));
    assert!(close(r.e4, (3.0 / 30.0 + 5.0 / 100.0) / 2.0));
    assert_eq!(r.e5, 1.0);
    assert!(close(
        r.eps,
        (0.0 + 12.0 / 30.0 + 0.4 + (0.1 + 0.05) / 2.0 + 1.0) / 5.0
    ));
}

#[test]
fn test_e1_threshold_and_e5_threshold() {
    let e = env();
    let prm = params(&e, 2.0);
    let (mut p, tx, tn) = arrs(2.0, 30.0, 15.0);
    p[40..50].fill(1.9);
    p[137..147].fill(0.0);
    p[137..142].fill(1.0);
    let r = occurrence_indicators(
        &pair((p, tx, tn)),
        &prm,
        e.cat.index_of("soja").unwrap(),
        40,
    )
    .unwrap();
    assert_eq!((r.e1, r.e5), (1.0, 0.0));
}

#[test]
fn test_sorgo_frost_is_strict_and_heat() {
    let e = env();
    let prm = params(&e, 5.0);
    let (p, mut tx, mut tn) = arrs(5.0, 30.0, 15.0);
    for t in [160, 161, 162] {
        tn[t] = 2.0;
    }
    for t in [170, 171] {
        tn[t] = 1.99;
    }
    for t in [210, 211, 212, 213] {
        tx[t] = 38.5;
    }
    let r = occurrence_indicators(
        &pair((p, tx, tn)),
        &prm,
        e.cat.index_of("sorgo_2a").unwrap(),
        147,
    )
    .unwrap();
    assert!(close(r.e4, (4.0 / 33.0 + 2.0 / 110.0) / 2.0));
}

#[test]
fn test_milho_has_only_frost_component() {
    let e = env();
    let prm = params(&e, 5.0);
    let (p, tx, mut tn) = arrs(5.0, 45.0, 15.0);
    for t in [160, 161] {
        tn[t] = 2.0;
    }
    let r = occurrence_indicators(
        &pair((p, tx, tn)),
        &prm,
        e.cat.index_of("milho_2a").unwrap(),
        147,
    )
    .unwrap();
    assert!(close(r.e4, 2.0 / 110.0));
}

#[test]
fn test_e3_zero_when_reference_zero_and_never_negative() {
    let e = env();
    let soy = e.cat.index_of("soja").unwrap();
    let wet = pair(arrs(50.0, 30.0, 15.0));
    assert_eq!(
        occurrence_indicators(&wet, &params(&e, 0.0), soy, 40)
            .unwrap()
            .e3,
        0.0
    );
    assert_eq!(
        occurrence_indicators(&wet, &params(&e, 1.0), soy, 40)
            .unwrap()
            .e3,
        0.0
    );
}

#[test]
fn test_e3_uses_same_window_in_second_agri_year() {
    let e = env();
    let soy = e.cat.index_of("soja").unwrap();
    let mut refy = vec![vec![1.0; 365]; 4];
    for row in &mut refy {
        row[97..127].fill(2.0);
    }
    let prm = load_params(&e.cfg, &e.cat)
        .with_reference(ConstructionClimatology::new(&refy, vec![1, 2, 3, 4]).unwrap());
    let r = occurrence_indicators(&pair(arrs(1.0, 30.0, 15.0)), &prm, soy, 405).unwrap();
    assert!(close(r.e3, 1.0 - 30.0 / 60.0));
}

#[test]
fn test_reference_median_is_numpy_median() {
    let mut x = Lcg(1);
    let rows: Vec<Vec<f64>> = (0..6)
        .map(|_| (0..365).map(|_| x.unif() * 10.0).collect())
        .collect();
    let cl = ConstructionClimatology::new(&rows, vec![]).unwrap();
    let mut sums: Vec<f64> = rows.iter().map(|r| np_sum(&r[100..130])).collect();
    sums.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let med = (sums[2] + sums[3]) / 2.0;
    assert_eq!(cl.median_window_sum(100, 130).unwrap(), med);
    let mut rows2 = rows.clone();
    for r in &mut rows2 {
        for v in &mut r[..5] {
            *v = 999.0;
        }
    }
    assert_eq!(
        ConstructionClimatology::new(&rows2, vec![])
            .unwrap()
            .median_window_sum(100, 130)
            .unwrap(),
        med
    );
    assert!(cl.median_window_sum(10, 10).is_err());
}

#[test]
fn test_missing_data_never_reaches_indicators() {
    let (mut p, tx, tn) = arrs(5.0, 30.0, 15.0);
    p[10] = f64::NAN;
    assert!(ClimatePair::new(p, tx, tn, "x")
        .unwrap_err()
        .0
        .contains("ausente"));
    assert!(ConstructionClimatology::new(&[vec![f64::NAN; 365]], vec![]).is_err());
    assert!(ClimatePair::new(vec![0.0; 729], vec![0.0; 729], vec![0.0; 729], "x").is_err());
}

#[test]
fn test_p0_values_wet_climate() {
    let e = env();
    let prm = params(&e, 5.0);
    let p0 = find(&e, ["soja", "milho_2a", "soja", "milho_2a"], Some((29, 29)));
    let g = loss_components(p0, &pair(arrs(5.0, 30.0, 15.0)), &prm).unwrap();
    assert!(close(g[0], 0.2));
    assert!(close(g[1], 0.5 * (310.0 / 730.0) + 0.5 * (148.0 / 730.0)));
    assert!(close(g[2], 2.0 / 3.0));
}

#[test]
fn test_dry_climate_all_failed() {
    let e = env();
    let prm = params(&e, 5.0);
    let p0 = find(&e, ["soja", "milho_2a", "soja", "milho_2a"], Some((29, 29)));
    let g = loss_components(p0, &pair(arrs(0.0, 30.0, 15.0)), &prm).unwrap();
    assert!(close(g[0], 0.6));
    assert!(close(g[1], 1.0));
}

#[test]
fn test_brachiaria_cover_hand_computed() {
    let e = env();
    let prm = params(&e, 5.0);
    let p = find(
        &e,
        ["soja", "milho_2a_braquiaria", "soja", "milho_2a_braquiaria"],
        Some((28, 28)),
    );
    let g = loss_components(p, &pair(arrs(5.0, 30.0, 15.0)), &prm).unwrap();
    assert!(close(g[1], 0.5 * (73.0 / 730.0) + 0.5 * (37.0 / 730.0)));
}

#[test]
fn test_establishment_failure_removes_cover_including_brachiaria() {
    let e = env();
    let prm = params(&e, 5.0);
    let p = find(
        &e,
        ["soja", "milho_2a_braquiaria", "soja", "milho_2a_braquiaria"],
        Some((28, 28)),
    );
    let (mut pr, tx, tn) = arrs(5.0, 30.0, 15.0);
    pr[137..147].fill(0.0);
    let g = loss_components(p, &pair((pr, tx, tn)), &prm).unwrap();
    assert!(close(g[1], 0.5 * (309.0 / 730.0) + 0.5 * (265.0 / 730.0)));
}

#[test]
fn test_g3_extremes() {
    let e = env();
    let prm = params(&e, 5.0);
    let diverse = find(
        &e,
        ["soja", "milho_2a_braquiaria", "milho_1a", "sorgo_2a"],
        None,
    );
    assert_eq!(diversity_terms(diverse, &prm).unwrap(), (4, 0));
    assert_eq!(diversity_g3(diverse, &prm).unwrap(), 0.0);
    let worst = find(
        &e,
        ["milho_1a", "milho_2a", "milho_1a", "milho_2a"],
        Some((30, 30)),
    );
    assert_eq!(diversity_terms(worst, &prm).unwrap(), (1, 3));
    assert!(close(diversity_g3(worst, &prm).unwrap(), 1.0));
}

#[test]
fn test_g1_is_mean_of_grain_occurrences_not_sum() {
    let e = env();
    let prm = params(&e, 5.0);
    let fall = e
        .plans
        .iter()
        .find(|p| p.fallow_windows().len() == 2)
        .unwrap();
    let pr = pair(arrs(5.0, 30.0, 15.0));
    let eps: Vec<f64> = fall
        .occupations
        .iter()
        .filter(|o| !o.is_fallow())
        .map(|o| {
            occurrence_indicators(&pr, &prm, o.crop, o.sow_t.unwrap())
                .unwrap()
                .eps
        })
        .collect();
    assert_eq!(eps.len(), 2);
    assert!(close(
        exposure_g1(fall, &pr, &prm).unwrap(),
        (eps[0] + eps[1]) / 2.0
    ));
    assert!(close(exposure_g1(fall, &pr, &prm).unwrap(), 0.2));
}

// ---------------------------------------------------------------- avaliador ingênuo independente
fn naive_loss(
    e: &Env,
    plan: &Plan,
    p: &[f64],
    tx: &[f64],
    tn: &[f64],
    refy: &[Vec<f64>],
    prm: &IndicatorParams,
) -> [f64; 3] {
    let mut covered = vec![false; T];
    let mut eps_list = Vec::new();
    for o in &plan.occupations {
        if o.is_fallow() {
            continue;
        }
        let crop = &e.cat.crops[o.crop];
        let s = o.sow_t.unwrap();
        let em = s + 7;
        let h = em + crop.cycle_days;
        let (p1, p2, p3) = (crop.phase_days[0], crop.phase_days[1], crop.phase_days[2]);
        let (c0, c1) = (em + p1 + p2, em + p1 + p2 + p3);
        let e1 = if p[s as usize..(s + 10) as usize].iter().sum::<f64>() < 20.0 {
            1.0
        } else {
            0.0
        };
        let (mut run, mut best) = (0, 0);
        for t in c0..c1 {
            run = if p[t as usize] < 1.0 { run + 1 } else { 0 };
            best = best.max(run);
        }
        let e2 = f64::from(best) / f64::from(p3);
        let y0 = if s >= 365 { 365 } else { 0 };
        let mut sums: Vec<f64> = refy
            .iter()
            .map(|r| np_sum(&r[(c0 - y0) as usize..(c1 - y0) as usize]))
            .collect();
        sums.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sums.len();
        let pref = if n % 2 == 1 {
            sums[n / 2]
        } else {
            (sums[n / 2 - 1] + sums[n / 2]) / 2.0
        };
        let pcrit = np_sum(&p[c0 as usize..c1 as usize]);
        let e3 = if pref == 0.0 {
            0.0
        } else {
            (1.0 - pcrit / pref).max(0.0)
        };
        let th = prm.thermal.get(&crop.key).copied().unwrap_or_default();
        let mut comps = Vec::new();
        if let Some(hc) = th.heat_c {
            comps.push((c0..c1).filter(|&t| tx[t as usize] > hc).count() as f64 / f64::from(p3));
        }
        if let Some(fc) = th.frost_c {
            let cnt = (em..h)
                .filter(|&t| {
                    if th.frost_strict {
                        tn[t as usize] < fc
                    } else {
                        tn[t as usize] <= fc
                    }
                })
                .count();
            comps.push(cnt as f64 / f64::from(crop.cycle_days));
        }
        let e4 = if comps.is_empty() {
            0.0
        } else {
            comps.iter().sum::<f64>() / comps.len() as f64
        };
        let e5 = if (h - 10..h).filter(|&t| p[t as usize] >= 1.0).count() >= 6 {
            1.0
        } else {
            0.0
        };
        eps_list.push((e1 + e2 + e3 + e4 + e5) / 5.0);
        if e1 == 0.0 {
            for t in em..h {
                covered[t as usize] = true;
            }
            if crop.brachiaria_cover_after_harvest {
                let end = if o.window == 3 {
                    T as i32
                } else {
                    (T as i32).min(plan.occupations[2].sow_t.unwrap() - 15)
                };
                for t in h..end {
                    covered[t as usize] = true;
                }
            }
        }
    }
    let g1 = eps_list.iter().sum::<f64>() / eps_list.len() as f64;
    let c1v = covered.iter().filter(|c| **c).count() as f64 / T as f64;
    let (mut run, mut best) = (0usize, 0usize);
    for c in &covered {
        run = if *c { 0 } else { run + 1 };
        best = best.max(run);
    }
    let g2 = 0.5 * (1.0 - c1v) + 0.5 * best as f64 / T as f64;
    let mut species: Vec<&str> = Vec::new();
    for o in &plan.occupations {
        for s in &e.cat.crops[o.crop].species {
            if !species.contains(&s.as_str()) {
                species.push(s);
            }
        }
    }
    let keys: Vec<&str> = plan
        .occupations
        .iter()
        .map(|o| e.cat.crops[o.crop].key.as_str())
        .collect();
    let mil = |k: &str| ["milho_1a", "milho_2a", "milho_2a_braquiaria"].contains(&k);
    let r1 = i32::from(keys[0] == keys[2]);
    let r2 = i32::from(keys[1] == keys[3] && keys[1] != "pousio");
    let r3 = i32::from((mil(keys[0]) && mil(keys[1])) || (mil(keys[2]) && mil(keys[3])));
    let g3 = 0.5 * (4.0 - species.len() as f64) / 3.0 + 0.5 * f64::from(r1 + r2 + r3) / 3.0;
    [g1, g2, g3]
}

fn check_vectorized_equals_naive(seed: u64) {
    let e = env();
    let mut rng = Lcg(seed);
    let mut gen = |n: usize| -> Vec<f64> {
        (0..n)
            .map(|_| {
                if rng.unif() < 0.45 {
                    -rng.unif().max(1e-9).ln() * 6.0
                } else {
                    0.0
                }
            })
            .collect()
    };
    let refy: Vec<Vec<f64>> = (0..7).map(|_| gen(365)).collect();
    let p = gen(T);
    let tx: Vec<f64> = (0..T).map(|_| rng.normal(31.0, 1.2)).collect();
    let tn: Vec<f64> = (0..T).map(|_| rng.normal(16.0, 1.4)).collect();
    let prm = load_params(&e.cfg, &e.cat)
        .with_reference(ConstructionClimatology::new(&refy, vec![]).unwrap());
    let pr = ClimatePair::new(p.clone(), tx.clone(), tn.clone(), "r").unwrap();
    let sample: Vec<&Plan> = e.plans.iter().step_by(e.plans.len() / 60).collect();
    assert!(sample.len() >= 50);
    for pl in sample {
        let got = loss_components(pl, &pr, &prm).unwrap();
        let want = naive_loss(&e, pl, &p, &tx, &tn, &refy, &prm);
        for k in 0..3 {
            assert!(
                (got[k] - want[k]).abs() < 1e-9,
                "{:?} g{}: {} vs {}",
                pl.id,
                k + 1,
                got[k],
                want[k]
            );
        }
    }
}

#[test]
fn test_vectorized_equals_naive_on_random_climates_11() {
    check_vectorized_equals_naive(11);
}

#[test]
fn test_vectorized_equals_naive_on_random_climates_12() {
    check_vectorized_equals_naive(12);
}

#[test]
fn test_vectorized_equals_naive_on_random_climates_13() {
    check_vectorized_equals_naive(13);
}

fn check_unit_interval(kind: &str) {
    let e = env();
    let prm = params(&e, 4.0);
    let mut rng = Lcg(5);
    let mut a = arrs(5.0, 30.0, 15.0);
    match kind {
        "zeros" => a.0 = vec![0.0; T],
        "flood" => a.0 = vec![500.0; T],
        "hot" => a.1 = vec![50.0; T],
        "cold" => a.2 = vec![-10.0; T],
        _ => {
            a.0 = (0..T)
                .map(|_| {
                    if rng.unif() < 0.3 {
                        rng.unif() * 40.0
                    } else {
                        0.0
                    }
                })
                .collect();
            a.1 = (0..T).map(|_| rng.normal(30.0, 3.0)).collect();
            a.2 = (0..T).map(|_| rng.normal(12.0, 3.0)).collect();
        }
    }
    let pr = pair(a);
    for p in &e.plans {
        let g = loss_components(p, &pr, &prm).unwrap();
        assert!(
            g.iter().all(|v| (0.0..=1.0).contains(v) && v.is_finite()),
            "{kind} {:?} {g:?}",
            p.id
        );
    }
}

#[test]
fn test_all_components_in_unit_interval_zeros() {
    check_unit_interval("zeros");
}

#[test]
fn test_all_components_in_unit_interval_flood() {
    check_unit_interval("flood");
}

#[test]
fn test_all_components_in_unit_interval_hot() {
    check_unit_interval("hot");
}

#[test]
fn test_all_components_in_unit_interval_cold() {
    check_unit_interval("cold");
}

#[test]
fn test_all_components_in_unit_interval_random() {
    check_unit_interval("random");
}

#[test]
fn test_pure_function_same_input_same_output() {
    let e = env();
    let prm = params(&e, 5.0);
    let mut rng = Lcg(9);
    let p: Vec<f64> = (0..T)
        .map(|_| {
            if rng.unif() < 0.4 {
                rng.unif() * 20.0
            } else {
                0.0
            }
        })
        .collect();
    let tx: Vec<f64> = (0..T).map(|_| rng.normal(31.0, 1.0)).collect();
    let tn: Vec<f64> = (0..T).map(|_| rng.normal(16.0, 1.0)).collect();
    let plan = &e.plans[100];
    let a = loss_components(
        plan,
        &ClimatePair::new(p.clone(), tx.clone(), tn.clone(), "a").unwrap(),
        &prm,
    )
    .unwrap();
    let pr = ClimatePair::new(p, tx, tn, "b").unwrap();
    let c: Vec<_> = (0..3)
        .map(|_| loss_components(plan, &pr, &prm).unwrap())
        .collect();
    assert!(c.iter().all(|x| *x == a));
}

#[test]
fn test_reference_required_for_e3() {
    let e = env();
    let no_ref = load_params(&e.cfg, &e.cat);
    let err = loss_components(&e.plans[100], &pair(arrs(5.0, 30.0, 15.0)), &no_ref).unwrap_err();
    assert!(err.0.contains("climatologia"));
}

#[test]
fn test_params_loaded_from_config() {
    let e = env();
    let prm = load_params(&e.cfg, &e.cat);
    assert_eq!(prm.default_weights, [0.5, 0.25, 0.25]);
    assert_eq!(prm.component_weights, [0.2; 5]);
    assert!(prm.thermal["sorgo_2a"].frost_strict);
    assert_eq!(prm.thermal["soja"].heat_c, Some(40.0));
    assert_eq!(prm.thermal["milho_1a"].heat_c, None);
    assert_eq!(prm.sha256.len(), 64);
}
