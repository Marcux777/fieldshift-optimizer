// AI-assisted implementation. Frozen evidence is read-only. Scientific recomputation is NOT ported yet.
mod selection;
mod pareto;

use std::{env, fs, net::SocketAddr, path::{Path, PathBuf}, sync::Arc};
use anyhow::{bail, Context, Result};
use axum::{extract::State, http::{HeaderValue, Method}, routing::get, Json, Router};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower_http::cors::CorsLayer;

const FROZEN_RUN: &str = "reports/20261114T181122Z";
const EXPECTED_PROTOCOL: &str = "1.1";
const EXPECTED_OUTCOME: &str = "NEGATIVO";

#[derive(Clone)]
struct AppState {
    report: Arc<Value>,
    run: Arc<Value>,
}
fn checked_json(dir: &Path, name: &str, sums: &str) -> Result<Value> {
    let line = sums.lines().find(|line| line.split_whitespace().nth(1) == Some(name))
        .with_context(|| format!("missing {name} in SHA256SUMS"))?;
    let expected = line.split_whitespace().next().context("missing digest")?;
    let bytes = fs::read(dir.join(name)).with_context(|| format!("missing {name}"))?;
    let actual = format!("{:x}", Sha256::digest(&bytes));
    if actual != expected { bail!("{name}: SHA-256 mismatch (frozen evidence must not be replaced)"); }
    serde_json::from_slice(&bytes).with_context(|| format!("invalid JSON in {name}"))
}
fn load_state(root: &Path) -> Result<AppState> {
    let dir = root.join(FROZEN_RUN);
    let sums = fs::read_to_string(dir.join("SHA256SUMS")).context("missing frozen SHA256SUMS")?;
    let report = checked_json(&dir, "eval_report.json", &sums)?;
    let run = checked_json(&dir, "run_result.json", &sums)?;
    if report["outcome"] != EXPECTED_OUTCOME || run["protocol_version"] != EXPECTED_PROTOCOL
        || run["status"] != "OK" || report["n_plans"] != run["n_plans"]
        || report["n_plans"] != 576 {
        bail!("unexpected frozen status, protocol, or candidate-plan count");
    }
    for policy in ["P0", "P1", "P1b", "P2"] {
        if report["policies"][policy].is_null() || run["policies"][policy].is_null() {
            bail!("missing policy {policy}");
        }
    }
    Ok(AppState { report: Arc::new(report), run: Arc::new(run) })
}
async fn health() -> Json<Value> {
    Json(json!({"status":"ok", "language":"Rust", "mode":"frozen-evidence", "recomputation_available":false}))
}
async fn summary(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "headline": s.report["headline"],
        "outcome": s.report["outcome"],
        "n_plans": s.report["n_plans"],
        "comparisons": s.report["comparisons"],
        "price_of_robustness": s.report["M5_price_of_robustness"],
        "weights": s.report["weights"],
        "protocol_version": s.run["protocol_version"],
        "snapshot_id": s.run["snapshot_id"],
        "run_result_hash": s.run["result_hash"],
        "evaluation_result_hash": s.report["result_hash"],
        "mode": "frozen-evidence",
        "can_optimize": false
    }))
}
async fn report(State(s): State<AppState>) -> Json<Value> { Json(s.report.as_ref().clone()) }
async fn run(State(s): State<AppState>) -> Json<Value> { Json(s.run.as_ref().clone()) }
async fn policies(State(s): State<AppState>) -> Json<Value> {
    Json(json!({"selection":s.run["policies"],"test":s.report["policies"]}))
}
async fn provenance(State(s): State<AppState>) -> Json<Value> {
    Json(json!({
        "snapshot_id": s.run["snapshot_id"],
        "protocol_version": s.run["protocol_version"],
        "config_hash": s.run["config_hash"],
        "construction": s.report["construction"],
        "test": s.report["test"],
        "limitations": s.report["limitations"],
        "verified_inputs": "SHA256SUMS of frozen report files",
        "recomputation_available": false
    }))
}
fn router(state: AppState, origin: HeaderValue) -> Router {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/v1/summary", get(summary))
        .route("/api/v1/report", get(report))
        .route("/api/v1/run", get(run))
        .route("/api/v1/policies", get(policies))
        .route("/api/v1/provenance", get(provenance))
        .with_state(state)
        .layer(CorsLayer::new().allow_origin(origin).allow_methods([Method::GET]))
}
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "fieldshift_api=info,tower_http=info".into())
    ).init();
    let root = env::var_os("FIELDSHIFT_ROOT").map(PathBuf::from).unwrap_or_else(|| {
        Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
    });
    let state = load_state(&root)?;
    let origin: HeaderValue = env::var("FRONTEND_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:5173".into()).parse()
        .context("invalid FRONTEND_ORIGIN")?;
    let host = env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".into());
    let addr: SocketAddr = host.parse().context("invalid BIND_ADDRESS")?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "serving frozen evidence; no reoptimization endpoint");
    axum::serve(listener, router(state, origin)).await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn loads_repository_frozen_evidence() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let s = load_state(root).expect("pinned frozen evidence is valid");
        assert_eq!(s.report["outcome"], "NEGATIVO");
        assert_eq!(s.run["n_plans"], 576);
        assert_eq!(s.run["protocol_version"], "1.1");
        assert!(s.report["comparisons"].as_array().unwrap().iter().any(|c|
            c["a"] == "P2" && c["b"] == "P1b" && c["classification"] == "NEGATIVO"));
    }
    #[test]
    fn frontier_matches_all_frozen_representatives() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let s = load_state(root).expect("verified run");
        let objs = s.run["objectives"].as_array().expect("objectives");
        let mut candidates = Vec::with_capacity(objs.len());
        for entry in objs {
            let row = entry.as_array().expect("row");
            let id = row[0].as_array().expect("plan ID").iter()
                .map(|x| x.as_i64().unwrap() as i32).collect();
            let mut scores = [0.0; 4];
            for (idx, output) in scores.iter_mut().enumerate() {
                *output = row[idx+1].as_f64().expect("normalized objective");
            }
            candidates.push(pareto::Candidate { id, scores });
        }
        let actual = pareto::frontier(&candidates).unwrap();
        let expected: Vec<Vec<i32>> = s.run["pareto"]["front_ids"].as_array().unwrap().iter()
            .map(|p| p.as_array().unwrap().iter().map(|i| i.as_i64().unwrap() as i32).collect())
            .collect();
        assert_eq!(actual, expected);
        assert_eq!(actual.len(), 36);
    }
    #[test]
    fn refuses_mutated_evidence() {
        let dir = std::env::temp_dir().join(format!("fieldshift-{}", std::process::id()));
        let frozen = dir.join(FROZEN_RUN);
        fs::create_dir_all(&frozen).unwrap();
        fs::write(frozen.join("SHA256SUMS"), "deadbeef  eval_report.json\n").unwrap();
        fs::write(frozen.join("eval_report.json"), "{}").unwrap();
        assert!(load_state(&dir).is_err());
        fs::remove_dir_all(dir).unwrap();
    }
}
