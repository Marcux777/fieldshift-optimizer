// AI-assisted (OpenBot agent)
use fieldshift_numeric::{calendar::Date, canon::*, num::*, rng::Pcg64};
use serde_json::Value;
fn fixture(name: &str) -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(format!(
            "{}/../../../fixtures/reference/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .expect("fixture"),
    )
    .expect("json")
}
fn vec(v: &Value) -> Vec<f64> {
    v.as_array()
        .expect("array")
        .iter()
        .map(|x| x.as_f64().expect("f64"))
        .collect()
}
fn eq(a: f64, b: f64) {
    assert_eq!(a.to_bits(), b.to_bits(), "{a} != {b}");
}
#[test]
fn rng_million_and_adverse_ranges() {
    let f = fixture("numeric_vectors.json");
    for r in f["rng"].as_array().expect("rng") {
        let a = Pcg64::from_seed(r["seed"].as_u64().expect("seed"))
            .integers(
                r["low"].as_i64().expect("low"),
                r["high"].as_i64().expect("high"),
                r["n"].as_u64().expect("n") as usize,
            )
            .expect("range");
        let bytes: Vec<u8> = a.iter().flat_map(|v| v.to_le_bytes()).collect();
        assert_eq!(sha256_hex(&bytes), r["sha256"].as_str().expect("hash"));
    }
    let a = Pcg64::from_seed(20261114)
        .integers(0, 7, 70000)
        .expect("range");
    assert_eq!(
        sha256_hex(&a.iter().flat_map(|v| v.to_le_bytes()).collect::<Vec<_>>()),
        f["bootstrap_sha256"].as_str().expect("hash")
    );
}
#[test]
fn numeric_exact() {
    let f = fixture("numeric_vectors.json");
    for r in f["round"].as_array().expect("round") {
        let x = r["x"].as_f64().expect("x");
        let nd = r["nd"].as_i64().expect("nd") as i32;
        eq(py_round(x, nd), r["py"].as_f64().expect("py"));
        eq(np_round(x, nd), r["np"].as_f64().expect("np"));
    }
    for r in f["sum"].as_array().expect("sum") {
        eq(fsum(&vec(&r["xs"])), r["result"].as_f64().expect("result"));
    }
    for r in f["quantile"].as_array().expect("q") {
        eq(
            quantile_linear(&vec(&r["xs"]), r["q"].as_f64().expect("q")),
            r["result"].as_f64().expect("result"),
        );
    }
    assert!(median(&[1., f64::NAN]).is_nan());
    assert!(nanmedian(&[f64::NAN]).is_nan());
    eq(nanmean(&[1., f64::NAN, 3.]), 2.);
}
#[test]
fn calendar_all_45_years() {
    for r in fixture("numeric_vectors.json")["calendar"]
        .as_array()
        .expect("dates")
    {
        let d = Date::parse(r["date"].as_str().expect("date")).expect("valid");
        assert_eq!(
            i64::from(d.ordinal()),
            r["ordinal"].as_i64().expect("ordinal")
        );
        assert_eq!(
            i64::from(d.agricultural_year()),
            r["agyear"].as_i64().expect("year")
        );
        assert_eq!(d.agricultural_day().map(|v| v as u64), r["agday"].as_u64());
    }
    assert!(Date::new(1900, 2, 29).is_err());
    assert!(Date::new(2000, 2, 29).is_ok());
}
#[test]
fn canonical_exact() {
    for r in fixture("canon_vectors.json").as_array().expect("cases") {
        assert_eq!(
            to_canonical(&r["value"]),
            r["ascii"].as_str().expect("ascii")
        );
        assert_eq!(
            to_canonical_utf8(&r["value"]),
            r["utf8"].as_str().expect("utf8")
        );
    }
}
