// AI-assisted (OpenBot agent)
use fieldshift_io::snapshot::*;
use fieldshift_numeric::canon::sha256_hex;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
fn real() -> PathBuf {
    latest_snapshot(&repo().join("data/snapshots")).expect("snapshot")
}
fn raw() -> Vec<u8> {
    use std::io::Read;
    let mut b = Vec::new();
    flate2::read::GzDecoder::new(
        fs::File::open(real().join("raw/power_daily.json.gz")).expect("raw"),
    )
    .read_to_end(&mut b)
    .expect("gzip");
    b
}
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "fs-io-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir(&p).expect("mkdir");
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("test cleanup");
    }
}
fn small() -> Value {
    let mut v: Value = serde_json::from_slice(&raw()).expect("json");
    for c in COLUMNS {
        let p = v["properties"]["parameter"][c]
            .as_object_mut()
            .expect("param");
        let keys: Vec<_> = p.keys().skip(40).cloned().collect();
        for k in keys {
            p.remove(&k);
        }
    }
    v
}
fn bytes(v: &Value) -> Vec<u8> {
    serde_json::to_vec(v).expect("encode")
}
#[test]
fn real_manifest_shape_quality_and_oracle() {
    let m = verify_manifest(&real()).expect("verify");
    assert_eq!(m["rows"], 16436);
    let c = load_climate(&real()).expect("load");
    assert_eq!(c.dates[0].iso(), "1981-01-01");
    assert_eq!(c.dates[16435].iso(), "2025-12-31");
    assert_eq!(c.quality_report()["leap_days_present"], 11);
    assert_eq!(c.quality_report()["ok"], true);
    let oracle: Value = serde_json::from_slice(
        &fs::read(repo().join("fixtures/reference/climate_daily.json")).expect("fixture"),
    )
    .expect("json");
    for (i, cname) in COLUMNS.iter().enumerate() {
        for (k, row) in c.values.iter().enumerate() {
            assert_eq!(
                row[i].to_bits(),
                oracle[cname][k].as_f64().expect("float").to_bits()
            );
        }
    }
}
#[test]
fn fill_and_null_stay_missing() {
    let mut v = small();
    v["properties"]["parameter"]["PRECTOTCORR"]["19810101"] = json!(-999.);
    v["properties"]["parameter"]["T2M"]["19810102"] = Value::Null;
    let p = parse_power_response(&bytes(&v)).expect("parse");
    assert!(p.climate.values[0][0].is_nan());
    assert!(p.climate.values[1][1].is_nan());
    assert_eq!(p.missing_fill_counts["PRECTOTCORR"], 1);
    assert_eq!(p.climate.quality_report()["ok"], false);
}
#[test]
fn duplicate_and_invalid_json() {
    assert!(strict_json(br#"{"a":{"x":1,"x":2}}"#).is_err());
    assert!(parse_power_response(b"<html>502</html>").is_err());
}
#[test]
fn units_fill_missing_extra_and_date_contracts() {
    let original = small();
    for kind in 0..7 {
        let mut v = original.clone();
        match kind {
            0 => v["header"]["fill_value"] = json!(-9999),
            1 => v["parameters"]["PRECTOTCORR"]["units"] = json!("mm/hour"),
            2 => {
                v["properties"]["parameter"]
                    .as_object_mut()
                    .expect("object")
                    .remove("T2M_MIN");
            }
            3 => v["properties"]["parameter"]["OTHER"] = json!({}),
            4 => {
                v["properties"]["parameter"]["T2M"]
                    .as_object_mut()
                    .expect("obj")
                    .remove("19810101");
            }
            5 => v["properties"]["parameter"]["PRECTOTCORR"]["19810101"] = json!(-3.),
            _ => v["properties"]["parameter"]["T2M_MIN"]["19810101"] = json!(100.),
        }
        assert!(parse_power_response(&bytes(&v)).is_err(), "kind {kind}");
    }
}
#[test]
fn frame_invariants_and_gaps() {
    let p = parse_power_response(&bytes(&small())).expect("parse");
    for kind in 0..5 {
        let mut c = p.climate.clone();
        match kind {
            0 => c.dates[1] = c.dates[0],
            1 => c.dates.swap(0, 1),
            2 => c.values[0][0] = -999.,
            3 => {
                c.dates.pop();
            }
            _ => c.values[0][1] = f64::INFINITY,
        }
        assert!(c.validate().is_err());
    }
    let mut c = p.climate;
    c.dates.remove(10);
    c.values.remove(10);
    assert_eq!(c.quality_report()["calendar_gaps"], 1);
}
#[test]
fn idempotence_volatile_times_and_changed_data() {
    let t = Temp::new();
    let mut v = small();
    let a =
        write_snapshot(&bytes(&v), &t.0, "test:offline", "2026-11-14T00:00:00Z").expect("write");
    let before = fs::read(a.join("manifest.json")).expect("manifest");
    v["times"] = json!({"data":999});
    let b =
        write_snapshot(&bytes(&v), &t.0, "test:offline", "2026-11-15T00:00:00Z").expect("again");
    assert_eq!(a, b);
    assert_eq!(before, fs::read(b.join("manifest.json")).expect("manifest"));
    v["properties"]["parameter"]["T2M"]["19810101"] = json!(42.);
    let c = write_snapshot(&bytes(&v), &t.0, "test:offline", "2026-11-14").expect("changed");
    assert_ne!(b, c);
    assert_eq!(fs::read_dir(&t.0).expect("list").count(), 2);
}
#[test]
fn altered_hash_missing_files_and_manifest() {
    for kind in 0..6 {
        let t = Temp::new();
        let p =
            write_snapshot(&bytes(&small()), &t.0, "test:offline", "2026-11-14").expect("snapshot");
        match kind {
            0 => fs::write(p.join("raw/power_daily.json.gz"), b"bad").expect("corrupt"),
            1 => {
                let mut v = read_manifest(&p).expect("manifest");
                v["files"]["raw"]["sha256"] = json!("0".repeat(64));
                fs::write(p.join("manifest.json"), bytes(&v)).expect("write");
            }
            2 => fs::write(
                p.join("SHA256SUMS"),
                format!("{}  manifest.json\n", "0".repeat(64)),
            )
            .expect("write"),
            3 => fs::remove_file(p.join("raw/power_daily.json.gz")).expect("remove"),
            4 => fs::remove_file(p.join("manifest.json")).expect("remove"),
            _ => {
                let mut m = read_manifest(&p).expect("manifest");
                m["files"]["raw"]["path"] = json!("../../escape");
                fs::write(p.join("manifest.json"), bytes(&m)).expect("write");
            }
        }
        assert!(verify_manifest(&p).is_err(), "kind {kind}");
        assert!(load_climate(&p).is_err());
    }
}
#[test]
fn latest_snapshot_rules() {
    let t = Temp::new();
    assert!(latest_snapshot(&t.0).is_err());
    let p = write_snapshot(&bytes(&small()), &t.0, "test:offline", "2026-11-14").expect("write");
    assert_eq!(latest_snapshot(&t.0).expect("one"), p);
    fs::write(t.0.join("current.txt"), "nonexistent").expect("current");
    assert!(latest_snapshot(&t.0).is_err());
    fs::write(t.0.join("current.txt"), "../escape").expect("current");
    assert!(latest_snapshot(&t.0).is_err());
}
#[test]
fn failed_write_leaves_no_partial_data() {
    let t = Temp::new();
    assert!(write_snapshot(b"{}", &t.0, "test:offline", "2026-11-14").is_err());
    assert_eq!(fs::read_dir(&t.0).expect("list").count(), 0);
}
#[test]
fn config_and_bom_zarc() {
    let r = repo();
    assert!(fieldshift_io::config::load_toml(&r.join("config/catalog.toml")).is_ok());
    let csv = fs::read_dir(r.join("config/zarc"))
        .expect("zarc")
        .map(|p| p.expect("entry").path())
        .find(|p| p.extension().is_some_and(|e| e == "csv"))
        .expect("csv");
    let rows = fieldshift_io::config::load_zarc(&csv).expect("parse");
    assert!(!rows.is_empty());
    assert!(rows[0].contains_key("Nome_cultura"));
    assert!(rows[0].contains_key("dec36"));
    assert_eq!(sha256_hex(&raw()).len(), 64);
}
