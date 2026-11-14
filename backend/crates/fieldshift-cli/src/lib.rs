// AI-assisted (OpenBot agent)
//! Composition root for the offline Rust command line. Domain calculations stay in model/eval.
use anyhow::{bail, Context, Result};
use fieldshift_eval::{
    adapter::{plan_checker, plan_infos, IndicatorLossModel},
    evaluation::{self, evaluate_plans, ClimateDaily, EvalInput, EvalProtocol},
};
use fieldshift_io::snapshot::{latest_snapshot, load_climate, verify_manifest};
use fieldshift_model::{
    catalog::{self, CatalogSources},
    indicators::IndicatorParams,
    optimization::{run, RunOptions},
    scenarios::ClimateFrame,
};
use fieldshift_numeric::canon::sha256_hex;
use serde_json::{json, Value};
use std::{collections::BTreeMap, fs, path::Path};
pub fn run_evaluation(root: &Path, snapshot: Option<&Path>) -> Result<(Value, Value, String)> {
    let protocol = fs::read(root.join("docs/EVALUATION_PROTOCOL.md"))?;
    if sha256_hex(&protocol) != evaluation::PROTOCOL_SHA256 {
        bail!("frozen protocol v1.1 changed");
    }
    let snap = match snapshot {
        Some(p) => p.to_path_buf(),
        None => latest_snapshot(&root.join("data/snapshots"))?,
    };
    let manifest = verify_manifest(&snap)?;
    let c = load_climate(&snap)?;
    let cat_bytes = fs::read(root.join("config/catalog.toml"))?;
    let soil_bytes = fs::read(root.join("config/soil.toml"))?;
    let cfg: toml::Value = toml::from_str(std::str::from_utf8(&cat_bytes)?)?;
    let zarc = cfg["zarc"]["file"].as_str().context("zarc file")?;
    if Path::new(zarc)
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        bail!("invalid zarc path");
    }
    let zarc_bytes = fs::read(root.join("config").join(zarc))?;
    let cat = catalog::load_catalog(&CatalogSources {
        catalog_toml: &cat_bytes,
        soil_toml: &soil_bytes,
        zarc_csv: &zarc_bytes,
    })?;
    let soil = catalog::load_soil(&soil_bytes, None, None)?;
    let params = IndicatorParams::from_toml(&fs::read(root.join("config/indicators.toml"))?, &cat)?;
    let frame = ClimateFrame::from_rows(
        c.dates
            .iter()
            .zip(&c.values)
            .map(|(d, v)| (d.year, d.month, d.day, *v)),
    )?;
    let sid = manifest["snapshot_id"].as_str().context("snapshot id")?;
    let rr = run(&frame, &cat, &soil, &params, sid, &RunOptions::default())?;
    let rr_value = rr.to_value(false);
    let (plans, _) = catalog::enumerate_plans(&cat, &soil, 0.20)?;
    let infos = plan_infos(&plans);
    let policy_ids: BTreeMap<_, _> = ["P0", "P1", "P1b", "P2"]
        .iter()
        .map(|p| {
            (
                p.to_string(),
                rr.policy_plan_id(p)
                    .map(|id| id.iter().map(|i| i64::from(*i)).collect()),
            )
        })
        .collect();
    let climate = ClimateDaily {
        dates: c
            .dates
            .iter()
            .map(|d| (d.year, u32::from(d.month), u32::from(d.day)))
            .collect(),
        prectotcorr: c.values.iter().map(|v| v[0]).collect(),
        t2m: c.values.iter().map(|v| v[1]).collect(),
        t2m_max: c.values.iter().map(|v| v[2]).collect(),
        t2m_min: c.values.iter().map(|v| v[3]).collect(),
    };
    let checker = plan_checker(&cat, &soil, &plans);
    let input = EvalInput {
        plans: &infos,
        policy_ids,
        p0_status: rr_value["policies"]["P0"]["status"]
            .as_str()
            .map(String::from),
        climate: &climate,
        weights: None,
        check_plan: Some(&checker),
        run_grid: true,
        optimizer_params_hash: rr_value["climatology"]["precip_sha256"]
            .as_str()
            .map(String::from),
    };
    let mut model = IndicatorLossModel::new(&plans, &params);
    let proto = EvalProtocol::default();
    let mut report = evaluate_plans(&input, &mut model, &proto)?;
    if rr.status() == "OK" {
        let ids: Vec<String> = infos
            .iter()
            .map(|p| {
                format!(
                    "[{}]",
                    p.id.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })
            .collect();
        report.audit_set(
            "x_hash_match",
            json!(
                sha256_hex(format!("[{}]", ids.join(", ")).as_bytes())
                    == rr_value["x_hash"].as_str().context("x hash")?
            ),
        );
        let split = evaluation::split_scenarios(&climate, &proto)?;
        let sb = evaluation::mean_climate(&split.construction_years.iter().collect::<Vec<_>>())?;
        report.audit_set(
            "sbar_hash_match",
            json!(
                sb.content_key()
                    == rr_value["climatology"]["sbar_sha256"]
                        .as_str()
                        .context("sbar hash")?
            ),
        );
        report.audit_set("optimizer_status", json!(rr.status()));
    }
    let hash = report.result_hash();
    let markdown = report.to_markdown();
    let mut ev = report.to_value();
    ev["result_hash"] = json!(hash);
    let mut out = rr_value;
    out["result_hash"] = json!(rr.result_hash());
    Ok((out, ev, markdown))
}
/// Differences in parsed values are scientific; text-only differences are serialization.
pub fn differences(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            for k in a
                .keys()
                .chain(b.keys())
                .collect::<std::collections::BTreeSet<_>>()
            {
                match (a.get(k), b.get(k)) {
                    (Some(a), Some(b)) => differences(a, b, &format!("{path}.{k}"), out),
                    _ => out.push(format!("{path}.{k}: missing key")),
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                out.push(format!("{path}: length {} != {}", a.len(), b.len()));
            }
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                differences(a, b, &format!("{path}[{i}]"), out);
            }
        }
        _ => {
            if a != b {
                out.push(format!("{path}: {a} != {b}"));
            }
        }
    }
}
