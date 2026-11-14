// AI-assisted (OpenBot agent)
//! E4/E5: cenários, componentes g1–g3, seleção nos 15 pesos, Pareto, alternativas e `RunResult`
//! reproduzem o Python (`16cc391`). Igualdade EXATA de bits nos floats (sem tolerância).

mod common;

use common::*;
use fieldshift_model::catalog::*;
use fieldshift_model::compat::{sha256_hex, to_canonical};
use fieldshift_model::indicators::{loss_components, ClimatePair};
use fieldshift_model::optimization::*;
use fieldshift_model::pareto::*;
use fieldshift_model::scenarios::*;
use fieldshift_model::selection::*;
use serde_json::Value;

fn f64s(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn bits_eq(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits()
}

#[test]
fn scenarios_split_and_climatology_match_python() {
    let clim = climate();
    let fx = fixture("scenarios.json");
    let c = build_scenarios(&clim, Split::Construction).unwrap();
    let t = build_scenarios(&clim, Split::Test).unwrap();
    for (set, key) in [(&c, "construction"), (&t, "test")] {
        let fs = fx[key].as_array().unwrap();
        assert_eq!(set.scenarios.len(), fs.len(), "{key}");
        for (s, f) in set.scenarios.iter().zip(fs) {
            assert_eq!(s.label, f["label"].as_str().unwrap());
            assert_eq!(s.y as i64, f["y"].as_i64().unwrap());
            assert_eq!(s.valid, f["valid"].as_bool().unwrap());
            assert_eq!(s.reason, f["reason"].as_str().unwrap());
        }
    }
    let (reference, sbar) = construction_climatology(&c).unwrap();
    let years: Vec<i64> = fx["climatology_years"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_i64().unwrap())
        .collect();
    assert_eq!(
        reference
            .labels
            .iter()
            .map(|y| i64::from(*y))
            .collect::<Vec<_>>(),
        years
    );
    assert_eq!(
        reference.sha256(),
        fx["climatology_precip_sha256"].as_str().unwrap()
    );
    let sb = &fx["sbar"];
    for (mine, key) in [(&sbar.p, "P"), (&sbar.tx, "TX"), (&sbar.tn, "TN")] {
        let want = f64s(&sb[key]);
        assert_eq!(mine.len(), want.len());
        for (d, (a, b)) in mine.iter().zip(&want).enumerate() {
            assert!(bits_eq(*a, *b), "s_barra {key}[{d}]: {a:?} != {b:?}");
        }
    }
    let sbar_bytes: Vec<u8> = [&sbar.p, &sbar.tx, &sbar.tn]
        .iter()
        .flat_map(|a| a.iter().flat_map(|v| v.to_le_bytes()))
        .collect();
    assert_eq!(sha256_hex(&sbar_bytes), fx["sbar_sha256"].as_str().unwrap());
    // pares de construção (primeiros 2): clima diário idêntico
    for (s, f) in c
        .valid()
        .iter()
        .zip(fx["construction_pairs"].as_array().unwrap())
    {
        let pr = s.pair.as_ref().unwrap();
        assert_eq!(s.label, f["label"].as_str().unwrap());
        for (mine, key) in [(&pr.p, "P"), (&pr.tx, "TX"), (&pr.tn, "TN")] {
            let want = f64s(&f[key]);
            assert!(
                mine.iter().zip(&want).all(|(a, b)| bits_eq(*a, *b)),
                "{} {key}",
                s.label
            );
        }
    }
    // medianas de e3 por janela (soma em pares + mediana)
    for (k, v) in fx["e3_median_window_sums"].as_object().unwrap() {
        let (lo, hi) = k.split_once(':').unwrap();
        let got = reference
            .median_window_sum(lo.parse().unwrap(), hi.parse().unwrap())
            .unwrap();
        assert!(
            bits_eq(got, v.as_f64().unwrap()),
            "mediana {k}: {got:?} != {v}"
        );
    }
}

fn load_pairs(clim: &ClimateFrame, split: Split) -> ScenarioSet {
    build_scenarios(clim, split).unwrap()
}

#[test]
fn components_match_python_bit_for_bit() {
    let cfgs = config();
    let cat = catalog(&cfgs);
    let soil = soil(&cfgs, None, None);
    let (plans, _) = enumerate_plans(&cat, &soil, 0.20).unwrap();
    let clim = climate();
    let cset = load_pairs(&clim, Split::Construction);
    let tset = load_pairs(&clim, Split::Test);
    let (reference, sbar) = construction_climatology(&cset).unwrap();
    let prm = params(&cfgs, &cat).with_reference(reference);
    let fx = fixture("components_default.json");
    let fids = fx["plan_ids"].as_array().unwrap();
    assert_eq!(fids.len(), plans.len());
    for (p, fid) in plans.iter().zip(fids) {
        assert_eq!(p.id.to_vec(), ids_of(fid));
    }
    let mut max_diff: f64 = 0.0;
    let mut mismatches = 0usize;
    let mut check = |got: [f64; 3], want: &Value, what: &str| {
        for (k, w) in want.as_array().unwrap().iter().enumerate() {
            let w = w.as_f64().unwrap();
            if !bits_eq(got[k], w) {
                mismatches += 1;
                max_diff = max_diff.max((got[k] - w).abs());
                if mismatches <= 5 {
                    eprintln!("DIVERGÊNCIA {what} g{}: {:?} != {w:?}", k + 1, got[k]);
                }
            }
        }
    };
    let cons: Vec<&ClimatePair> = cset
        .valid()
        .iter()
        .map(|s| s.pair.as_ref().unwrap())
        .collect();
    let test: Vec<&ClimatePair> = tset
        .valid()
        .iter()
        .map(|s| s.pair.as_ref().unwrap())
        .collect();
    for (i, p) in plans.iter().enumerate() {
        for (j, pair) in cons.iter().enumerate() {
            check(
                loss_components(p, pair, &prm).unwrap(),
                &fx["construction"][i][j],
                "construção",
            );
        }
        for (j, pair) in test.iter().enumerate() {
            check(
                loss_components(p, pair, &prm).unwrap(),
                &fx["test"][i][j],
                "teste",
            );
        }
        check(
            loss_components(p, &sbar, &prm).unwrap(),
            &fx["sbar"][i],
            "s_barra",
        );
    }
    assert_eq!(
        mismatches, 0,
        "componentes divergem do Python: max |Δ| = {max_diff:e}"
    );
}

fn selection_inputs() -> (Vec<Plan>, Tensor3, Vec<[f64; 3]>) {
    let cfgs = config();
    let cat = catalog(&cfgs);
    let soil = soil(&cfgs, None, None);
    let (plans, _) = enumerate_plans(&cat, &soil, 0.20).unwrap();
    let clim = climate();
    let cset = build_scenarios(&clim, Split::Construction).unwrap();
    let (reference, sbar) = construction_climatology(&cset).unwrap();
    let prm = params(&cfgs, &cat).with_reference(reference);
    let pairs: Vec<&ClimatePair> = cset
        .valid()
        .iter()
        .map(|s| s.pair.as_ref().unwrap())
        .collect();
    let comp = components_tensor(&plans, &pairs, &prm).unwrap();
    let bar: Vec<[f64; 3]> = plans
        .iter()
        .map(|p| loss_components(p, &sbar, &prm).unwrap())
        .collect();
    (plans, comp, bar)
}

#[test]
fn selections_in_all_15_weight_vectors_match_python() {
    let (plans, comp, bar) = selection_inputs();
    let ids: Vec<PlanId> = plans.iter().map(|p| p.id).collect();
    let fx = fixture("selections_15w.json");
    let rows = fx["rows"].as_array().unwrap();
    let grid = weight_grid();
    assert_eq!(rows.len(), 15);
    for (w, row) in grid.iter().zip(rows) {
        assert_eq!(f64s(&row["weights"]), w.to_vec());
        let l = loss_matrix(&comp, *w);
        let lb = loss_vector(&bar, *w);
        let r = regret_matrix(&l);
        // vetores intermediários (chaves de desempate), antes de q()
        let (lm, mr) = (mean_over_scenarios(&l), mean_over_scenarios(&r));
        let mx = row_max(&r);
        for (name, mine) in [
            ("L_sbar", &lb),
            ("max_R", &mx),
            ("mean_L_fsum", &lm),
            ("mean_R_fsum", &mr),
        ] {
            let want = f64s(&row["keys"][name]);
            for (i, (a, b)) in mine.iter().zip(&want).enumerate() {
                assert!(bits_eq(*a, *b), "w={w:?} {name}[{i}]: {a:?} != {b:?}");
            }
        }
        for (name, sel) in [
            ("P1", select_conv(&l, &lb, &ids)),
            ("P1b", select_exp(&l, &ids)),
            ("P2", select_mmr(&l, &ids)),
        ] {
            let f = &row[name];
            assert_eq!(
                sel.index as i64,
                f["index"].as_i64().unwrap(),
                "w={w:?} {name}"
            );
            assert_eq!(
                ids[sel.index].to_vec(),
                ids_of(&f["plan_id"]),
                "w={w:?} {name}"
            );
            let tied: Vec<i64> = f["tied_after_level"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_i64().unwrap())
                .collect();
            assert_eq!(
                sel.tied_after_level
                    .iter()
                    .map(|x| *x as i64)
                    .collect::<Vec<_>>(),
                tied,
                "w={w:?} {name}"
            );
            assert_eq!(
                sel.id_tiebreak,
                f["id_tiebreak"].as_bool().unwrap(),
                "w={w:?} {name}"
            );
        }
    }
}

#[test]
fn pareto_and_alternatives_match_python() {
    let (plans, comp, bar) = selection_inputs();
    let ids: Vec<PlanId> = plans.iter().map(|p| p.id).collect();
    let fx = fixture("pareto.json");
    let f = objective_vectors(&comp);
    for (i, row) in fx["F"].as_array().unwrap().iter().enumerate() {
        let want = f64s(row);
        for k in 0..4 {
            assert!(
                bits_eq(f[i][k], want[k]),
                "F[{i}][{k}]: {:?} != {:?}",
                f[i][k],
                want[k]
            );
        }
    }
    let (front, eq) = pareto_front(&f, &ids);
    assert_eq!(
        front.iter().map(|&i| ids[i].to_vec()).collect::<Vec<_>>(),
        fx["front_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(ids_of)
            .collect::<Vec<_>>()
    );
    assert_eq!(front.len() as i64, fx["n_front_vectors"].as_i64().unwrap());
    let n_plans: usize = front.iter().map(|r| 1 + eq[r].len()).sum();
    assert_eq!(n_plans as i64, fx["n_front_plans"].as_i64().unwrap());
    for (k, v) in fx["equivalents"].as_object().unwrap() {
        let r: usize = k.parse().unwrap();
        let mine: Vec<i64> = eq[&r].iter().map(|&i| i as i64).collect();
        let want: Vec<i64> = v
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_i64().unwrap())
            .collect();
        assert_eq!(mine, want, "equivalentes de {r}");
    }
    // alternativas: as 4 políticas do peso padrão + extremos
    let w = DEFAULT_WEIGHTS;
    let l = loss_matrix(&comp, w);
    let lb = loss_vector(&bar, w);
    let cfgs = config();
    let cat = catalog(&cfgs);
    let soil = soil(&cfgs, None, None);
    let (p0, _) = resolve_spec(&cat, &soil, &cat.default_p0, &plans, 0.20).unwrap();
    let i0 = plan_index(&plans, &p0.unwrap().id).unwrap();
    let picks = vec![
        ("P0".to_string(), i0),
        ("P1".to_string(), select_conv(&l, &lb, &ids).index),
        ("P1b".to_string(), select_exp(&l, &ids).index),
        ("P2".to_string(), select_mmr(&l, &ids).index),
    ];
    let alts = select_alternatives(&plans, &f, &front, &picks, w, K_ALTERNATIVES, MIN_HAMMING);
    let want = fx["alternatives"].as_array().unwrap();
    assert_eq!(alts.len(), want.len());
    for (a, fa) in alts.iter().zip(want) {
        assert_eq!(a.plan_id.to_vec(), ids_of(&fa["plan_id"]));
        let labels: Vec<&str> = fa["labels"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect();
        assert_eq!(a.labels, labels);
        assert_eq!(a.reason, fa["reason"].as_str().unwrap());
        assert_eq!(a.on_front, fa["on_front"].as_bool().unwrap());
        assert_eq!(a.hamming_relaxed, fa["hamming_relaxed"].as_bool().unwrap());
        assert_eq!(
            a.min_hamming_to_others as i64,
            fa["min_hamming_to_others"].as_i64().unwrap()
        );
    }
}

/// Resultado completo (`RunResult`) idêntico ao golden do Python, campo a campo e por hash.
#[test]
fn run_result_equals_golden_and_hash() {
    let cfgs = config();
    let cat = catalog(&cfgs);
    let soil = soil(&cfgs, None, None);
    let prm = params(&cfgs, &cat);
    let golden = fixture("golden_run_result.json");
    let manifest = fixture("MANIFEST.json");
    let snapshot_id = golden["snapshot_id"].as_str().unwrap();
    let res = run(
        &climate(),
        &cat,
        &soil,
        &prm,
        snapshot_id,
        &RunOptions::default(),
    )
    .unwrap();
    assert_eq!(res.status(), "OK");
    let mine = res.to_value(false);
    let g = golden.as_object().unwrap();
    for (k, gv) in g {
        if k == "meta" || k == "result_hash" {
            continue;
        }
        assert_eq!(&mine[k], gv, "campo `{k}` diverge do golden");
    }
    assert_eq!(
        mine.as_object().unwrap().len(),
        g.len() - 2,
        "conjunto de campos"
    );
    let want_hash = manifest["golden"]["run_result_hash"].as_str().unwrap();
    assert_eq!(golden["result_hash"].as_str().unwrap(), want_hash);
    assert_eq!(
        res.result_hash(),
        want_hash,
        "result_hash (JSON canônico estilo Python)"
    );
    assert_eq!(sha256_hex(to_canonical(&mine).as_bytes()), want_hash);
}
