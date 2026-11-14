// AI-assisted (OpenBot agent)
//! Offline raw POWER reader. Parquet is hashed for frozen snapshots, never used as source.
use fieldshift_numeric::{
    calendar::Date,
    canon::{sha256_hex, to_canonical_utf8},
};
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use serde::{
    de::{self, MapAccess, SeqAccess, Visitor},
    Deserialize,
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
pub const COLUMNS: [&str; 4] = ["PRECTOTCORR", "T2M", "T2M_MAX", "T2M_MIN"];
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct SnapshotError(pub String);
type Res<T> = Result<T, SnapshotError>;
fn error(s: impl ToString) -> SnapshotError {
    SnapshotError(s.to_string())
}

struct Strict(Value);
impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = Strict;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "JSON without duplicate keys")
            }
            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Strict, E> {
                serde_json::Number::from_f64(v)
                    .map(|n| Strict(Value::Number(n)))
                    .ok_or_else(|| E::custom("non-finite JSON"))
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Strict, E> {
                Ok(Strict(v.into()))
            }
            fn visit_unit<E: de::Error>(self) -> Result<Strict, E> {
                Ok(Strict(Value::Null))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut out = Vec::new();
                while let Some(Strict(v)) = a.next_element()? {
                    out.push(v);
                }
                Ok(Strict(Value::Array(out)))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Strict, A::Error> {
                let mut out = serde_json::Map::new();
                while let Some((k, Strict(v))) = a.next_entry::<String, Strict>()? {
                    if out.insert(k.clone(), v).is_some() {
                        return Err(de::Error::custom(format!("chave duplicada: {k}")));
                    }
                }
                Ok(Strict(Value::Object(out)))
            }
        }
        d.deserialize_any(V)
    }
}
pub fn strict_json(raw: &[u8]) -> Res<Value> {
    serde_json::from_slice::<Strict>(raw)
        .map(|v| v.0)
        .map_err(error)
}
#[derive(Debug, Clone)]
pub struct ClimateData {
    pub dates: Vec<Date>,
    pub values: Vec<[f64; 4]>,
}
#[derive(Debug, Clone)]
pub struct ParsedResponse {
    pub climate: ClimateData,
    pub document: Value,
    pub missing_fill_counts: BTreeMap<String, usize>,
}
impl ClimateData {
    pub fn validate(&self) -> Res<()> {
        if self.dates.len() != self.values.len() || self.dates.is_empty() {
            return Err(error("clima vazio ou dimensões incompatíveis"));
        }
        for i in 0..self.dates.len() {
            if i > 0 && self.dates[i] <= self.dates[i - 1] {
                return Err(error("datas duplicadas ou fora de ordem"));
            }
            let v = self.values[i];
            if v.contains(&-999.) {
                return Err(error("-999 não convertido"));
            }
            if v.iter().any(|x| x.is_infinite()) {
                return Err(error("valor infinito"));
            }
            if v[0] < 0. {
                return Err(error("PRECTOTCORR negativa"));
            }
            if v[3] > v[2] {
                return Err(error("T2M_MIN > T2M_MAX"));
            }
        }
        Ok(())
    }
    pub fn quality_report(&self) -> Value {
        let mut gaps = Vec::new();
        let mut gap_count = 0;
        for pair in self.dates.windows(2) {
            let n = pair[1].ordinal() - pair[0].ordinal() - 1;
            gap_count += n;
            if n > 0 {
                gaps.push(json!({"after":pair[0].iso(),"missing_days":n}));
            }
        }
        let missing: BTreeMap<_, _> = COLUMNS
            .iter()
            .enumerate()
            .map(|(i, c)| (*c, self.values.iter().filter(|r| r[i].is_nan()).count()))
            .collect();
        let ranges: BTreeMap<_, _> = COLUMNS
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let v: Vec<_> = self
                    .values
                    .iter()
                    .map(|r| r[i])
                    .filter(|x| x.is_finite())
                    .collect();
                (
                    *c,
                    if v.is_empty() {
                        Value::Null
                    } else {
                        json!([
                            v.iter().copied().fold(f64::INFINITY, f64::min),
                            v.iter().copied().fold(f64::NEG_INFINITY, f64::max)
                        ])
                    },
                )
            })
            .collect();
        json!({"rows":self.dates.len(),"date_min":self.dates.first().map(|d|d.iso()),"date_max":self.dates.last().map(|d|d.iso()),"calendar_gaps":gap_count,"gap_spans":gaps,"duplicate_dates":self.dates.windows(2).filter(|d|d[0]==d[1]).count(),"monotonic":self.dates.windows(2).all(|d|d[0]<d[1]),"missing_by_column":missing,"value_ranges":ranges,"leap_days_present":self.dates.iter().filter(|d|d.month==2&&d.day==29).count(),"ok":self.validate().is_ok()&&gap_count==0&&missing.values().all(|n|*n==0)})
    }
}
pub fn parse_power_response(raw: &[u8]) -> Res<ParsedResponse> {
    let doc = strict_json(raw)?;
    if doc["header"]["fill_value"].as_f64() != Some(-999.) {
        return Err(error("fill_value inesperado"));
    }
    let params = doc["properties"]["parameter"]
        .as_object()
        .ok_or_else(|| error("parâmetros ausentes"))?;
    if params.len() != 4 || COLUMNS.iter().any(|c| !params.contains_key(*c)) {
        return Err(error("parâmetros ausentes ou não previstos"));
    }
    if doc["header"]["api"]["version"].as_str().is_none()
        || doc["geometry"]["coordinates"]
            .as_array()
            .is_none_or(|v| v.len() < 2)
    {
        return Err(error("resposta POWER fora do formato"));
    }
    let mut columns = Vec::new();
    for c in COLUMNS {
        let unit = if c == "PRECTOTCORR" { "mm/day" } else { "C" };
        if doc["parameters"][c]["units"].as_str() != Some(unit) {
            return Err(error(format!("unidade inesperada: {c}")));
        }
        columns.push(
            params[c]
                .as_object()
                .ok_or_else(|| error(format!("datas inválidas em {c}")))?,
        );
    }
    let keys: Vec<_> = columns[0].keys().collect();
    if columns.iter().any(|c| c.keys().collect::<Vec<_>>() != keys) {
        return Err(error("parâmetros têm datas diferentes"));
    }
    let mut climate = ClimateData {
        dates: Vec::new(),
        values: Vec::new(),
    };
    let mut missing = [0usize; 4];
    for key in keys {
        let d = Date::parse(key).map_err(error)?;
        let mut row = [0.; 4];
        for i in 0..4 {
            let v = &columns[i][key];
            row[i] = if v.is_null() {
                f64::NAN
            } else {
                v.as_f64()
                    .ok_or_else(|| error(format!("valor não numérico em {} {key}", COLUMNS[i])))?
            };
            if row[i] == -999. {
                row[i] = f64::NAN;
                missing[i] += 1;
            }
        }
        climate.dates.push(d);
        climate.values.push(row);
    }
    climate.validate()?;
    Ok(ParsedResponse {
        climate,
        missing_fill_counts: COLUMNS
            .iter()
            .enumerate()
            .map(|(i, c)| (c.to_string(), missing[i]))
            .collect(),
        document: doc,
    })
}
pub fn content_sha256(raw: &[u8]) -> Res<String> {
    let mut doc = strict_json(raw)?;
    doc.as_object_mut()
        .ok_or_else(|| error("POWER document must be object"))?
        .remove("times");
    Ok(sha256_hex(to_canonical_utf8(&doc).as_bytes()))
}
fn local_path(root: &Path, s: &str) -> Res<PathBuf> {
    let p = Path::new(s);
    if p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err(error("manifest path must be relative without traversal"));
    }
    let path = root.join(p);
    if path.exists()
        && !path
            .canonicalize()
            .map_err(error)?
            .starts_with(root.canonicalize().map_err(error)?)
    {
        return Err(error("manifest path escapes snapshot"));
    }
    Ok(path)
}
pub fn read_manifest(root: &Path) -> Res<Value> {
    strict_json(&fs::read(root.join("manifest.json")).map_err(error)?)
}
pub fn verify_manifest(root: &Path) -> Res<Value> {
    let m = read_manifest(root)?;
    if !matches!(m["manifest_schema"].as_u64(), Some(1 | 2)) {
        return Err(error("manifest_schema inesperado"));
    }
    let files = m["files"]
        .as_object()
        .ok_or_else(|| error("manifesto sem files"))?;
    if !files.contains_key("raw") || (m["manifest_schema"] == 1 && !files.contains_key("parquet")) {
        return Err(error("manifesto sem raw/parquet"));
    }
    for entry in files.values() {
        let path = local_path(
            root,
            entry["path"]
                .as_str()
                .ok_or_else(|| error("arquivo sem path"))?,
        )?;
        let b = fs::read(&path).map_err(error)?;
        if Some(sha256_hex(&b).as_str()) != entry["sha256"].as_str() {
            return Err(error(format!("hash divergente em {}", path.display())));
        }
    }
    let raw_entry = &m["files"]["raw"];
    let gz = fs::read(local_path(
        root,
        raw_entry["path"]
            .as_str()
            .ok_or_else(|| error("raw path"))?,
    )?)
    .map_err(error)?;
    let mut raw = Vec::new();
    GzDecoder::new(&gz[..])
        .take(64 * 1024 * 1024 + 1)
        .read_to_end(&mut raw)
        .map_err(error)?;
    if raw.len() > 64 * 1024 * 1024 {
        return Err(error("raw exceeds 64 MiB"));
    }
    if raw_entry["uncompressed_sha256"].as_str() != Some(sha256_hex(&raw).as_str())
        || raw_entry["content_sha256"].as_str() != Some(content_sha256(&raw)?.as_str())
    {
        return Err(error("hash da resposta descomprimida/conteúdo diverge"));
    }
    let sid = m["snapshot_id"]
        .as_str()
        .ok_or_else(|| error("snapshot_id ausente"))?;
    if root.file_name().and_then(|v| v.to_str()) != Some(sid)
        || !sid.ends_with(&content_sha256(&raw)?[..12])
    {
        return Err(error("snapshot_id não corresponde ao diretório/conteúdo"));
    }
    let sums = root.join("SHA256SUMS");
    if sums.is_file() {
        for line in fs::read_to_string(sums).map_err(error)?.lines() {
            let (hash, name) = line
                .split_once("  ")
                .ok_or_else(|| error("SHA256SUMS inválido"))?;
            if sha256_hex(&fs::read(local_path(root, name)?).map_err(error)?) != hash {
                return Err(error(format!("SHA256SUMS diverge em {name}")));
            }
        }
    }
    let p = parse_power_response(&raw)?;
    if m["rows"].as_u64() != Some(p.climate.dates.len() as u64)
        || m["date_min"].as_str() != p.climate.dates.first().map(|d| d.iso()).as_deref()
        || m["date_max"].as_str() != p.climate.dates.last().map(|d| d.iso()).as_deref()
    {
        return Err(error("linhas ou datas divergem do manifesto"));
    }
    for c in COLUMNS {
        if m["units"][c] != p.document["parameters"][c]["units"] {
            return Err(error("unidades divergem do manifesto"));
        }
    }
    Ok(m)
}
pub fn load_climate(root: &Path) -> Res<ClimateData> {
    let m = verify_manifest(root)?;
    let b = fs::read(local_path(
        root,
        m["files"]["raw"]["path"]
            .as_str()
            .ok_or_else(|| error("raw path"))?,
    )?)
    .map_err(error)?;
    let mut raw = Vec::new();
    GzDecoder::new(&b[..])
        .read_to_end(&mut raw)
        .map_err(error)?;
    Ok(parse_power_response(&raw)?.climate)
}
pub fn latest_snapshot(root: &Path) -> Res<PathBuf> {
    let current = root.join("current.txt");
    if current.is_file() {
        let p = local_path(root, fs::read_to_string(current).map_err(error)?.trim())?;
        if !p.is_dir() {
            return Err(error("current.txt aponta para snapshot inexistente"));
        }
        return Ok(p);
    }
    let mut found = Vec::new();
    for e in fs::read_dir(root).map_err(error)? {
        let e = e.map_err(error)?;
        if e.file_type().map_err(error)?.is_dir()
            && !e.file_name().to_string_lossy().starts_with('.')
        {
            found.push(e.path());
        }
    }
    if found.len() != 1 {
        return Err(error(format!(
            "esperado 1 snapshot, encontrei {}",
            found.len()
        )));
    }
    Ok(found.remove(0))
}
/// Rust snapshots use schema 2 (raw only); frozen schema 1 files are never rewritten.
pub fn write_snapshot(
    raw: &[u8],
    root: &Path,
    request_url: &str,
    collected_utc: &str,
) -> Res<PathBuf> {
    let p = parse_power_response(raw)?;
    let hash = content_sha256(raw)?;
    let geometry = &p.document["geometry"]["coordinates"];
    let version = p.document["header"]["api"]["version"]
        .as_str()
        .ok_or_else(|| error("version"))?;
    let sid = format!(
        "power-daily-{}-lat{}_lon{}-{}",
        version.trim_start_matches('v'),
        geometry[1].as_f64().ok_or_else(|| error("latitude"))?,
        geometry[0].as_f64().ok_or_else(|| error("longitude"))?,
        &hash[..12]
    );
    fs::create_dir_all(root).map_err(error)?;
    let out = root.join(&sid);
    if out.exists() {
        verify_manifest(&out)?;
        return Ok(out);
    }
    let tmp = root.join(format!(".tmp-{sid}-{}", std::process::id()));
    fs::create_dir(&tmp).map_err(error)?;
    let result = (|| {
        fs::create_dir(tmp.join("raw")).map_err(error)?;
        let mut gz = GzEncoder::new(Vec::new(), Compression::best());
        gz.write_all(raw).map_err(error)?;
        let gz = gz.finish().map_err(error)?;
        fs::write(tmp.join("raw/power_daily.json.gz"), &gz).map_err(error)?;
        let m = json!({"manifest_schema":2,"snapshot_id":sid,"collected_utc":collected_utc,"source":{"service":"NASA POWER Daily API","api_version":version,"sources":p.document["header"]["sources"],"time_standard":p.document["header"]["time_standard"]},"request":{"url":request_url,"parameters":COLUMNS,"returned_geometry":geometry},"fill_value":-999.,"units":COLUMNS.iter().map(|c|(c.to_string(),p.document["parameters"][c]["units"].clone())).collect::<serde_json::Map<_,_>>(),"rows":p.climate.dates.len(),"date_min":p.climate.dates.first().map(|d|d.iso()),"date_max":p.climate.dates.last().map(|d|d.iso()),"missing_fill_counts":p.missing_fill_counts,"known_gaps":p.climate.quality_report()["gap_spans"],"files":{"raw":{"path":"raw/power_daily.json.gz","sha256":sha256_hex(&gz),"bytes":gz.len(),"uncompressed_sha256":sha256_hex(raw),"uncompressed_bytes":raw.len(),"content_sha256":hash}},"citation":{"reference":"NASA Langley Research Center Prediction Of Worldwide Energy Resources (POWER) project, NASA Earth Science Division","data_reference":format!("POWER Project Daily {version}, accessed {collected_utc}"),"referencing_guide":"https://power.larc.nasa.gov/docs/referencing/"},"tool_versions":{"fieldshift":env!("CARGO_PKG_VERSION")}});
        fs::write(
            tmp.join("manifest.json"),
            serde_json::to_vec_pretty(&m).map_err(error)?,
        )
        .map_err(error)?;
        let mut sums = String::new();
        for name in ["manifest.json", "raw/power_daily.json.gz"] {
            sums.push_str(&format!(
                "{}  {name}\n",
                sha256_hex(&fs::read(tmp.join(name)).map_err(error)?)
            ));
        }
        fs::write(tmp.join("SHA256SUMS"), sums).map_err(error)?;
        fs::rename(&tmp, &out).map_err(error)?;
        verify_manifest(&out)?;
        Ok(out.clone())
    })();
    if result.is_err() && tmp.exists() {
        fs::remove_dir_all(&tmp).map_err(error)?;
    }
    result
}
