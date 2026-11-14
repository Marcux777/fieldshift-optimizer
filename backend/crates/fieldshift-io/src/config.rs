// AI-assisted (OpenBot agent)
//! Configuration files are data; domain interpretation stays in fieldshift-model.
use crate::snapshot::SnapshotError;
use std::{collections::BTreeMap, fs, path::Path};
pub fn load_toml(path: &Path) -> Result<toml::Value, SnapshotError> {
    let s =
        fs::read_to_string(path).map_err(|e| SnapshotError(format!("{}: {e}", path.display())))?;
    toml::from_str(&s).map_err(|e| SnapshotError(format!("{}: {e}", path.display())))
}
pub fn load_zarc(path: &Path) -> Result<Vec<BTreeMap<String, String>>, SnapshotError> {
    let b = fs::read(path).map_err(|e| SnapshotError(e.to_string()))?;
    let b = b.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(&b);
    let mut r = csv::ReaderBuilder::new().delimiter(b';').from_reader(b);
    let h = r
        .headers()
        .map_err(|e| SnapshotError(e.to_string()))?
        .clone();
    for d in 1..=36 {
        if !h.iter().any(|s| s == format!("dec{d}")) {
            return Err(SnapshotError(format!("ZARC missing dec{d}")));
        }
    }
    r.records()
        .map(|rec| {
            let rec = rec.map_err(|e| SnapshotError(e.to_string()))?;
            Ok(h.iter()
                .zip(rec.iter())
                .map(|(k, v)| (k.into(), v.into()))
                .collect())
        })
        .collect()
}
