// AI-assisted (OpenBot agent)
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};
fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}
#[test]
fn cli_recomputes_and_classifies_reference() {
    let out = std::env::temp_dir().join(format!("fieldshift-cli-{}", std::process::id()));
    assert!(!out.exists());
    let bin = env!("CARGO_BIN_EXE_fieldshift-cli");
    let result = Command::new(bin)
        .current_dir(repo())
        .args(["run-evaluation", "--output"])
        .arg(&out)
        .output()
        .expect("execute");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let m: Value = serde_json::from_slice(&result.stdout).expect("manifest");
    assert_eq!(m["outcome"], "NEGATIVO");
    assert_eq!(
        m["eval_result_hash"],
        "504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18"
    );
    assert_eq!(
        m["run_result_hash"],
        "b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66"
    );
    for (name, golden) in [
        ("run_result.json", "golden_run_result.json"),
        ("eval_report.json", "golden_eval_report.json"),
    ] {
        let r = Command::new(bin)
            .current_dir(repo())
            .arg("diff-reference")
            .arg(out.join(name))
            .arg(repo().join("fixtures/reference").join(golden))
            .output()
            .expect("diff");
        assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stdout));
    }
    let mut changed: Value =
        serde_json::from_slice(&fs::read(out.join("eval_report.json")).expect("report"))
            .expect("json");
    changed["outcome"] = "POSITIVO".into();
    fs::write(
        out.join("changed.json"),
        serde_json::to_vec(&changed).expect("json"),
    )
    .expect("write");
    let r = Command::new(bin)
        .current_dir(repo())
        .arg("diff-reference")
        .arg(out.join("changed.json"))
        .arg(out.join("eval_report.json"))
        .output()
        .expect("diff");
    assert!(!r.status.success());
    assert!(String::from_utf8_lossy(&r.stdout).contains("SCIENTIFIC"));
    let repeat = Command::new(bin)
        .current_dir(repo())
        .arg("collect-snapshot")
        .output()
        .expect("offline collect");
    assert!(repeat.status.success());
    assert!(String::from_utf8_lossy(&repeat.stdout).contains("offline"));
    fs::remove_dir_all(out).expect("cleanup");
}
