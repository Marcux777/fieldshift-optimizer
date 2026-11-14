// AI-assisted (OpenBot agent)
//! R1: o fluxo do bootstrap tem de ser o de `np.random.default_rng(20261114).integers(0, 7, (10000, 7))`.
use fieldshift_eval::npcompat::Pcg64;
use std::path::PathBuf;

fn fixture(name: &str) -> serde_json::Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/reference")
        .join(name);
    serde_json::from_str(&std::fs::read_to_string(p).expect("fixture")).expect("json")
}

#[test]
fn bootstrap_indices_bit_exact_against_numpy() {
    let f = fixture("eval_intermediates.json");
    let b = &f["bootstrap"];
    let seed = b["seed"].as_u64().expect("seed");
    let n_boot = b["n_boot"].as_u64().expect("n") as usize;
    let nb = b["n_blocks"].as_u64().expect("nb") as u32;
    let want: Vec<u64> = b["indices"]
        .as_array()
        .expect("rows")
        .iter()
        .flat_map(|r| {
            r.as_array()
                .expect("row")
                .iter()
                .map(|v| v.as_u64().expect("int"))
        })
        .collect();
    let mut g = Pcg64::from_seed(seed);
    let got: Vec<u64> = g
        .integers_below(nb, n_boot * nb as usize)
        .into_iter()
        .map(u64::from)
        .collect();
    assert_eq!(got.len(), want.len());
    let first_bad = got.iter().zip(&want).position(|(a, b)| a != b);
    assert_eq!(
        first_bad, None,
        "primeira divergência no índice {first_bad:?}"
    );
}
