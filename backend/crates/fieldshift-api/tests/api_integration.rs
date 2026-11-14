// AI-assisted (OpenBot agent)
//! Integration, contract, and differential tests for `fieldshift-api` (E7).
//!
//! Verifies all HTTP endpoints, boundary validation, error status codes,
//! bit-for-bit parity against `fixtures/reference/` (E0), and the ported
//! edge cases from `tests/test_pipeline.py`.

use std::path::PathBuf;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use fieldshift_api::{
    engine::{CurrentPlan, Engine, Inputs},
    resolve_bind_address, router, router_with_engine,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn load_fixture(name: &str) -> Value {
    let path = repo_root().join("fixtures/reference").join(name);
    let raw = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()));
    serde_json::from_str(&raw).expect("valid fixture JSON")
}

fn test_app() -> Router {
    let engine = Engine::load(&repo_root()).expect("engine loads from verified root");
    router(engine, None, Vec::new())
}

async fn send_request(app: &Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(req)
        .await
        .expect("router handles request");
    let status = resp.status();
    let bytes = resp
        .into_body()
        .collect()
        .await
        .expect("body bytes")
        .to_bytes();
    let val = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({ "raw": String::from_utf8_lossy(&bytes) }));
    (status, val)
}

async fn get_json(app: &Router, uri: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::GET)
        .uri(uri)
        .body(Body::empty())
        .expect("request");
    send_request(app, req).await
}

async fn post_json(app: &Router, uri: &str, body: &Value) -> (StatusCode, Value) {
    let req = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(body).expect("json bytes")))
        .expect("request");
    send_request(app, req).await
}

fn diff_json(path: &str, actual: &Value, expected: &Value, out: &mut Vec<String>) {
    match (actual, expected) {
        (Value::Object(a), Value::Object(b)) => {
            for k in a.keys().chain(b.keys().filter(|k| !a.contains_key(*k))) {
                diff_json(
                    &format!("{path}.{k}"),
                    a.get(k).unwrap_or(&Value::Null),
                    b.get(k).unwrap_or(&Value::Null),
                    out,
                );
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for (i, (x, y)) in a.iter().zip(b.iter()).enumerate() {
                diff_json(&format!("{path}[{i}]"), x, y, out);
            }
        }
        _ if actual != expected => {
            out.push(format!("{path}: actual={actual} expected={expected}"));
        }
        _ => {}
    }
}

#[tokio::test]
async fn health_meta_catalog_provenance_limitations_and_openapi() {
    let app = test_app();
    let golden_run = load_fixture("golden_run_result.json");

    for path in ["/health", "/v1/health"] {
        let (status, health) = get_json(&app, path).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            health,
            json!({
                "status": "ok",
                "language": "Rust",
                "mode": "computed",
                "recomputation_available": true
            })
        );
    }

    let (status, meta) = get_json(&app, "/v1/meta").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(meta["language"], "Rust");
    assert_eq!(meta["mode"], "computed");
    assert_eq!(meta["protocol_version"], "1.1");
    assert_eq!(meta["snapshot_id"], golden_run["snapshot_id"]);
    assert_eq!(meta["config_hash"], golden_run["config_hash"]);
    assert_eq!(
        meta["protocol_sha256"],
        "ee8e51c5b2eecc7319818a5645ef15b84d677eabab35caad1a7e9fc41deaefb9"
    );

    let (status, cat) = get_json(&app, "/v1/catalog").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cat["crops"].as_array().expect("crops").len(), 6);
    assert_eq!(cat["windows"].as_array().expect("windows").len(), 4);
    assert_eq!(
        cat["soils"],
        json!(["AD1", "AD2", "AD3", "AD4", "AD5", "AD6"])
    );
    assert_eq!(cat["textures"], json!(["arenosa", "media", "argilosa"]));
    assert_eq!(
        cat["default_p0"],
        json!({
            "j1": "soja",
            "dec1": 29,
            "j2": "milho_2a",
            "j3": "soja",
            "dec3": 29,
            "j4": "milho_2a"
        })
    );
    assert_eq!(cat["weight_grid"].as_array().expect("grid").len(), 15);
    assert_eq!(cat["risk_limit"], json!(0.2));

    let (status, prov) = get_json(&app, "/v1/provenance").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(prov["manifest"]["snapshot_id"], golden_run["snapshot_id"]);
    assert_eq!(prov["manifest"]["source"]["api_version"], "v2.10.3");
    assert_eq!(
        prov["protocol_sha256"],
        "ee8e51c5b2eecc7319818a5645ef15b84d677eabab35caad1a7e9fc41deaefb9"
    );

    let (status, lim) = get_json(&app, "/v1/limitations").await;
    assert_eq!(status, StatusCode::OK);
    let items = lim["items"].as_array().expect("items");
    assert_eq!(items.len(), 10);
    assert!(items[0]
        .as_str()
        .expect("str")
        .contains("does not replace the farmer"));

    let (status, spec) = get_json(&app, "/v1/openapi.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(spec["openapi"], "3.0.3");
    for route in [
        "/v1/health",
        "/v1/meta",
        "/v1/catalog",
        "/v1/plans",
        "/v1/run",
        "/v1/evaluate",
        "/v1/pareto",
        "/v1/provenance",
        "/v1/limitations",
    ] {
        assert!(
            spec["paths"].get(route).is_some(),
            "missing route in openapi.json: {route}"
        );
    }
}

#[tokio::test]
async fn plans_endpoint_matches_all_8_soil_fixtures() {
    let app = test_app();
    let cases = [
        ("AD1", "argilosa", "catalog_AD1_argilosa.json", 169usize),
        ("AD2", "argilosa", "catalog_AD2_argilosa.json", 289),
        ("AD3", "argilosa", "catalog_AD3_argilosa.json", 400),
        ("AD4", "arenosa", "catalog_AD4_arenosa.json", 576),
        ("AD4", "media", "catalog_AD4_media.json", 576),
        ("AD4", "argilosa", "catalog_AD4_argilosa.json", 576),
        ("AD5", "argilosa", "catalog_AD5_argilosa.json", 729),
        ("AD6", "argilosa", "catalog_AD6_argilosa.json", 961),
    ];

    for (ad, tex, fixture_name, expected_n) in cases {
        let (status, body) = post_json(
            &app,
            "/v1/plans",
            &json!({
                "ad_class": ad,
                "texture": tex,
                "weights": [0.5, 0.25, 0.25]
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "failed on {ad}/{tex}");
        assert_eq!(body["status"], "OK");
        assert_eq!(body["n_plans"], expected_n, "count mismatch on {ad}/{tex}");

        let fx = load_fixture(fixture_name);
        let actual_ids: Vec<Value> = body["plans"]
            .as_array()
            .expect("plans")
            .iter()
            .map(|p| p["id"].clone())
            .collect();
        let expected_ids: Vec<Value> = fx["plans"]
            .as_array()
            .expect("fx plans")
            .iter()
            .map(|p| p["id"].clone())
            .collect();
        assert_eq!(actual_ids, expected_ids, "IDs/order mismatch on {ad}/{tex}");
    }
}

#[tokio::test]
async fn run_and_pareto_match_golden_run_result_bit_for_bit() {
    let app = test_app();
    let mut golden = load_fixture("golden_run_result.json");

    let (status, mut run_resp) = post_json(&app, "/v1/run", &json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(run_resp["status"], "OK");
    assert_eq!(run_resp["n_plans"], 576);
    assert_eq!(
        run_resp["result_hash"],
        "b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66"
    );

    // Explicitly passing default_p0 produces the exact same config_hash and result_hash.
    let (status_explicit, run_explicit) = post_json(
        &app,
        "/v1/run",
        &json!({
            "ad_class": "AD4",
            "texture": "argilosa",
            "weights": [0.5, 0.25, 0.25],
            "p0": {
                "j1": "soja",
                "dec1": 29,
                "j2": "milho_2a",
                "j3": "soja",
                "dec3": 29,
                "j4": "milho_2a"
            }
        }),
    )
    .await;
    assert_eq!(status_explicit, StatusCode::OK);
    assert_eq!(run_explicit["config_hash"], run_resp["config_hash"]);
    assert_eq!(run_explicit["result_hash"], run_resp["result_hash"]);

    // Compare all scientific fields (excluding runtime meta).
    run_resp.as_object_mut().expect("obj").remove("meta");
    golden.as_object_mut().expect("obj").remove("meta");
    let mut diffs = Vec::new();
    diff_json("$", &run_resp, &golden, &mut diffs);
    assert!(
        diffs.is_empty(),
        "RunResult differed from golden fixture: {:#?}",
        &diffs[..diffs.len().min(15)]
    );

    let (status_p, pareto_resp) = post_json(&app, "/v1/pareto", &json!({})).await;
    assert_eq!(status_p, StatusCode::OK);
    assert_eq!(pareto_resp["pareto"], golden["pareto"]);
    assert_eq!(pareto_resp["objectives"], golden["objectives"]);
    assert_eq!(pareto_resp["alternatives"], golden["alternatives"]);
}

#[tokio::test]
async fn evaluate_matches_golden_eval_report_bit_for_bit_and_ad1_p0_diagnosis() {
    let app = test_app();
    let golden = load_fixture("golden_eval_report.json");

    let (status, eval_resp) = post_json(&app, "/v1/evaluate", &json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(eval_resp["outcome"], "NEGATIVO");
    assert_eq!(
        eval_resp["result_hash"],
        "504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18"
    );

    let mut diffs = Vec::new();
    diff_json("$", &eval_resp, &golden, &mut diffs);
    assert!(
        diffs.is_empty(),
        "EvalReport differed from golden fixture: {:#?}",
        &diffs[..diffs.len().min(15)]
    );

    // AD1 exploratory case: P0 is infeasible under ZARC 20%, but 169 feasible plans remain.
    let ad1_body = json!({
        "ad_class": "AD1",
        "texture": "argilosa",
        "weights": [0.5, 0.25, 0.25]
    });
    let (status_r, ad1_run) = post_json(&app, "/v1/run", &ad1_body).await;
    assert_eq!(status_r, StatusCode::OK);
    assert_eq!(ad1_run["n_plans"], 169);
    assert_eq!(ad1_run["policies"]["P0"]["status"], "INVIAVEL_SOB_ZARC");
    let reasons = ad1_run["policies"]["P0"]["reasons"]
        .as_array()
        .expect("reasons");
    assert!(reasons
        .iter()
        .any(|r| r.as_str().unwrap_or_default().contains("risco 30% > 20%")));

    let (status_e, ad1_eval) = post_json(&app, "/v1/evaluate", &ad1_body).await;
    assert_eq!(status_e, StatusCode::OK);
    assert_eq!(ad1_eval["n_plans"], 169);
    assert_eq!(ad1_eval["p0_status"], "inviavel_sob_ZARC_20");
    assert_eq!(ad1_eval["policies"]["P0"]["feasible"], false);
    assert_eq!(ad1_eval["policies"]["P1"]["feasible"], true);
    assert_eq!(ad1_eval["policies"]["P1b"]["feasible"], true);
    assert_eq!(ad1_eval["policies"]["P2"]["feasible"], true);
}

#[tokio::test]
async fn boundary_validation_and_ported_pipeline_cases() {
    // 1. Invalid weights fail validation before touching engine data (ported from test_pipeline.py).
    for invalid_weights in [
        [0.5, 0.5, 0.5],
        [f64::NAN, 0.5, 0.5],
        [-1.0, 1.0, 1.0],
        [f64::INFINITY, 0.0, 0.0],
    ] {
        let inp = Inputs {
            weights: invalid_weights,
            ..Inputs::default()
        };
        let err = inp.validate().expect_err("invalid weights must fail");
        assert!(err.contains("weights"));
    }

    // 2. Missing snapshot is not replaced by synthetic fallback.
    let tmp = std::env::temp_dir().join("fs-api-missing-snapshot-test");
    let _ = std::fs::remove_dir_all(&tmp);
    assert!(Engine::load(&tmp).is_err());

    // 3. HTTP boundary validation: 422, 400, 415, 413, 404, 405.
    let app = test_app();
    for bad_payload in [
        json!({ "ad_class": "AD9", "texture": "argilosa", "weights": [0.5, 0.25, 0.25] }),
        json!({ "ad_class": "AD4", "texture": "silte", "weights": [0.5, 0.25, 0.25] }),
        json!({ "ad_class": "AD4", "texture": "argilosa", "weights": [0.9, 0.25, 0.25] }),
        json!({ "ad_class": "AD4", "texture": "argilosa", "weights": [-0.5, 1.0, 0.5] }),
        json!({
            "ad_class": "AD4",
            "texture": "argilosa",
            "weights": [0.5, 0.25, 0.25],
            "p0": { "j1": "soja", "dec1": 37, "j2": "milho_2a", "j3": "soja", "dec3": 29, "j4": "milho_2a" }
        }),
        json!({
            "ad_class": "AD4",
            "texture": "argilosa",
            "weights": [0.5, 0.25, 0.25],
            "p0": { "j1": "trigo", "dec1": 29, "j2": "milho_2a", "j3": "soja", "dec3": 29, "j4": "milho_2a" }
        }),
        json!({ "unknown_field": 123 }),
    ] {
        let (st, body) = post_json(&app, "/v1/run", &bad_payload).await;
        assert_eq!(
            st,
            StatusCode::UNPROCESSABLE_ENTITY,
            "payload: {bad_payload}"
        );
        assert!(body["error"]["code"].is_string());
    }

    // Malformed JSON -> 400
    let bad_json_req = Request::builder()
        .method(Method::POST)
        .uri("/v1/run")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{broken"))
        .expect("req");
    let (st_bad_json, body_bad_json) = send_request(&app, bad_json_req).await;
    assert_eq!(st_bad_json, StatusCode::BAD_REQUEST);
    assert_eq!(body_bad_json["error"]["code"], "INVALID_JSON");

    // Wrong Content-Type -> 415
    let bad_ct_req = Request::builder()
        .method(Method::POST)
        .uri("/v1/run")
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from("{}"))
        .expect("req");
    let (st_ct, _) = send_request(&app, bad_ct_req).await;
    assert_eq!(st_ct, StatusCode::UNSUPPORTED_MEDIA_TYPE);

    // Payload > 64 KiB -> 413
    let huge = format!("{{\"ad_class\":\"{}\"}}", "A".repeat(70 * 1024));
    let huge_req = Request::builder()
        .method(Method::POST)
        .uri("/v1/run")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(huge))
        .expect("req");
    let (st_huge, _) = send_request(&app, huge_req).await;
    assert_eq!(st_huge, StatusCode::PAYLOAD_TOO_LARGE);

    // Unknown route -> 404, wrong method -> 405
    let (st_404, body_404) = get_json(&app, "/v1/nonexistent").await;
    assert_eq!(st_404, StatusCode::NOT_FOUND);
    assert_eq!(body_404["error"]["code"], "NOT_FOUND");

    let (st_405, body_405) = get_json(&app, "/v1/run").await;
    assert_eq!(st_405, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body_405["error"]["code"], "METHOD_NOT_ALLOWED");

    // 4. Insufficient climate is explicit (ported from test_pipeline.py::test_insufficient_climate_is_explicit).
    let short_engine = Engine::load(&repo_root())
        .expect("load")
        .with_truncated_climate(10)
        .expect("truncate");
    let short_app = router(short_engine, None, Vec::new());
    let (st_sr, short_run) = post_json(&short_app, "/v1/run", &json!({})).await;
    assert_eq!(st_sr, StatusCode::OK);
    assert_eq!(short_run["status"], "DADOS_INSUFICIENTES");

    let (st_se, short_eval) = post_json(&short_app, "/v1/evaluate", &json!({})).await;
    assert_eq!(st_se, StatusCode::OK);
    assert_eq!(short_eval["outcome"], "DADOS_INSUFICIENTES");
    assert_eq!(short_eval["policies"], json!({}));

    // 5. Empty feasible set is explicit (ported from test_pipeline.py::test_empty_feasible_set_is_explicit).
    let inf_engine = Engine::load(&repo_root())
        .expect("load")
        .with_infeasible_zarc();
    let inf_app = router(inf_engine, None, Vec::new());
    let (st_ir, inf_run) = post_json(&inf_app, "/v1/run", &json!({})).await;
    assert_eq!(st_ir, StatusCode::OK);
    assert_eq!(inf_run["status"], "INVIAVEL");
    assert_eq!(inf_run["n_plans"], 0);

    let (st_ie, inf_eval) = post_json(&inf_app, "/v1/evaluate", &json!({})).await;
    assert_eq!(st_ie, StatusCode::OK);
    assert_eq!(inf_eval["outcome"], "INVIAVEL");
    assert_eq!(inf_eval["policies"], json!({}));

    let p0 = CurrentPlan {
        j1: "soja".into(),
        dec1: 29,
        j2: "milho_2a".into(),
        j3: "soja".into(),
        dec3: 29,
        j4: "milho_2a_braquiaria".into(),
    };
    assert_eq!(
        Inputs {
            p0: Some(p0),
            ..Inputs::default()
        }
        .validate(),
        Ok(())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_default_requests_single_flight_and_bind_address_resolution() {
    use std::sync::Arc;

    // O3: BIND_ADDRESS precedence over PORT, and 0.0.0.0:$PORT fallback.
    assert_eq!(
        resolve_bind_address(Some("127.0.0.1:3100"), Some("10000")),
        "127.0.0.1:3100"
    );
    assert_eq!(resolve_bind_address(None, Some("10000")), "0.0.0.0:10000");
    assert_eq!(
        resolve_bind_address(Some("  "), Some("4100")),
        "0.0.0.0:4100"
    );
    assert_eq!(resolve_bind_address(None, None), "127.0.0.1:3000");

    // O1 + O2: 16 concurrent page-load bursts with default inputs (both implicit `{}` and explicit default_p0)
    // must never return 429 BUSY and must deduplicate heavy computations via single-flight + deterministic cache,
    // without mixing configurations or masking invalid inputs.
    let engine = Arc::new(Engine::load(&repo_root()).expect("engine loads"));
    let app = router_with_engine(engine.clone(), None, Vec::new());

    // 1. /v1/meta uses precomputed default_config_hash without triggering any engine run.
    let (st_meta, meta_body) = get_json(&app, "/v1/meta").await;
    assert_eq!(st_meta, StatusCode::OK);
    assert_eq!(
        meta_body["config_hash"],
        load_fixture("golden_run_result.json")["config_hash"]
    );
    assert_eq!(engine.call_counts(), (0, 0, 0));

    // 2. Fire 16 concurrent bursts (each doing /v1/meta, /v1/run {}, /v1/run explicit, /v1/evaluate {},
    //    /v1/evaluate explicit, /v1/plans {}, /v1/pareto {}) plus a concurrent non-default AD1 request
    //    and an invalid AD9 request.
    let mut handles = Vec::new();
    for idx in 0..16usize {
        let app_c = app.clone();
        handles.push(tokio::spawn(async move {
            let explicit_default = json!({
                "ad_class": "AD4",
                "texture": "argilosa",
                "weights": [0.5, 0.25, 0.25],
                "p0": {
                    "j1": "soja",
                    "dec1": 29,
                    "j2": "milho_2a",
                    "j3": "soja",
                    "dec3": 29,
                    "j4": "milho_2a"
                }
            });
            let run_payload = if idx % 2 == 0 {
                json!({})
            } else {
                explicit_default.clone()
            };
            let eval_payload = if idx % 2 == 1 {
                json!({})
            } else {
                explicit_default
            };

            let (m_st, m_val) = get_json(&app_c, "/v1/meta").await;
            let (r_st, r_val) = post_json(&app_c, "/v1/run", &run_payload).await;
            let (e_st, e_val) = post_json(&app_c, "/v1/evaluate", &eval_payload).await;
            let (p_st, p_val) = post_json(&app_c, "/v1/plans", &json!({})).await;
            let (pa_st, pa_val) = post_json(&app_c, "/v1/pareto", &json!({})).await;

            assert_eq!(m_st, StatusCode::OK, "burst {idx} meta failed");
            assert_eq!(r_st, StatusCode::OK, "burst {idx} run failed: {r_val}");
            assert_eq!(e_st, StatusCode::OK, "burst {idx} evaluate failed: {e_val}");
            assert_eq!(p_st, StatusCode::OK, "burst {idx} plans failed: {p_val}");
            assert_eq!(pa_st, StatusCode::OK, "burst {idx} pareto failed: {pa_val}");

            assert_eq!(
                r_val["result_hash"],
                "b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66"
            );
            assert_eq!(
                e_val["result_hash"],
                "504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18"
            );
            assert_eq!(e_val["outcome"], "NEGATIVO");
            assert_eq!(p_val["n_plans"], 576);
            assert_eq!(pa_val["pareto"]["n_front"], 36);
            assert_eq!(m_val["config_hash"], r_val["config_hash"]);
        }));
    }

    let app_ad1 = app.clone();
    let ad1_handle = tokio::spawn(async move {
        post_json(
            &app_ad1,
            "/v1/plans",
            &json!({
                "ad_class": "AD1",
                "texture": "argilosa",
                "weights": [0.5, 0.25, 0.25]
            }),
        )
        .await
    });

    let app_bad = app.clone();
    let bad_handle = tokio::spawn(async move {
        post_json(
            &app_bad,
            "/v1/run",
            &json!({
                "ad_class": "AD9",
                "texture": "argilosa",
                "weights": [0.5, 0.25, 0.25]
            }),
        )
        .await
    });

    for h in handles {
        h.await.expect("concurrent default task panicked");
    }
    let (st_ad1, val_ad1) = ad1_handle.await.expect("ad1 task");
    assert_eq!(st_ad1, StatusCode::OK);
    assert_eq!(val_ad1["n_plans"], 169);

    let (st_bad, val_bad) = bad_handle.await.expect("bad task");
    assert_eq!(st_bad, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(val_bad["error"]["code"], "INVALID_INPUT");

    // Verify single-flight deduplication: across 16 concurrent bursts (32 run/pareto calls, 16 evaluate calls,
    // 16 plans calls + 1 AD1 plans call), default evaluate ran once, default plans ran once (+1 AD1 = 2),
    // and engine.run ran twice (1 for default run/pareto + 1 inside default evaluate).
    let (run_calls, evaluate_calls, plans_calls) = engine.call_counts();
    assert_eq!(evaluate_calls, 1, "default evaluate must run exactly once");
    assert_eq!(
        run_calls, 2,
        "default run must run once for run/pareto and once inside evaluate"
    );
    assert_eq!(
        plans_calls, 2,
        "plans must run once for default AD4 and once for non-default AD1"
    );
}

/// Item A1: concurrent NON-default inputs bypass the default-input cache. With two compute slots they
/// used to get an immediate `429 BUSY`; they must now wait for a slot and all succeed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_non_default_requests_wait_for_a_slot_instead_of_429() {
    let app = test_app();
    let bodies = [
        json!({"ad_class": "AD1", "texture": "argilosa", "weights": [0.5, 0.25, 0.25]}),
        json!({"ad_class": "AD2", "texture": "argilosa", "weights": [0.5, 0.25, 0.25]}),
        json!({"ad_class": "AD3", "texture": "media", "weights": [0.5, 0.25, 0.25]}),
        json!({"ad_class": "AD5", "texture": "argilosa", "weights": [0.5, 0.25, 0.25]}),
        json!({"ad_class": "AD4", "texture": "argilosa", "weights": [0.75, 0.25, 0.0]}),
        json!({"ad_class": "AD4", "texture": "argilosa", "weights": [0.25, 0.25, 0.5]}),
    ];
    let mut handles = Vec::new();
    for body in bodies {
        let app_c = app.clone();
        handles.push(tokio::spawn(async move {
            post_json(&app_c, "/v1/run", &body).await
        }));
    }
    let mut statuses = Vec::new();
    for h in handles {
        let (st, val) = h.await.expect("task");
        statuses.push((st, val["error"]["code"].clone()));
    }
    assert!(
        statuses.iter().all(|(st, _)| *st == StatusCode::OK),
        "all concurrent non-default runs must succeed, got {statuses:?}"
    );
}

/// Item A1 policy: the wait for a slot is bounded. When it runs out the API answers `503 BUSY`
/// with `Retry-After` instead of holding the connection (protects small instances such as Render Free).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bounded_queue_wait_returns_503_with_retry_after() {
    use fieldshift_api::{router_with_limits, ApiLimits, RETRY_AFTER_SECS};
    use std::{sync::Arc, time::Duration};

    let engine = Arc::new(Engine::load(&repo_root()).expect("engine loads"));
    let app = router_with_limits(
        engine,
        None,
        Vec::new(),
        ApiLimits {
            slots: 1,
            queue_wait: Duration::from_millis(50),
        },
    );
    let mut handles = Vec::new();
    for ad in ["AD1", "AD2", "AD3"] {
        let app_c = app.clone();
        handles.push(tokio::spawn(async move {
            let body = json!({"ad_class": ad, "texture": "argilosa", "weights": [0.5, 0.25, 0.25]});
            let req = Request::builder()
                .method(Method::POST)
                .uri("/v1/run")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&body).expect("json")))
                .expect("request");
            let resp = app_c.oneshot(req).await.expect("response");
            let status = resp.status();
            let retry = resp
                .headers()
                .get(header::RETRY_AFTER)
                .map(|v| v.to_str().expect("ascii").to_string());
            let bytes = resp.into_body().collect().await.expect("body").to_bytes();
            let val: Value = serde_json::from_slice(&bytes).expect("json body");
            (status, retry, val)
        }));
    }
    let mut ok = 0;
    let mut busy = 0;
    for h in handles {
        let (status, retry, val) = h.await.expect("task");
        match status {
            StatusCode::OK => ok += 1,
            StatusCode::SERVICE_UNAVAILABLE => {
                busy += 1;
                assert_eq!(val["error"]["code"], "BUSY");
                assert_eq!(retry, Some(RETRY_AFTER_SECS.to_string()));
            }
            other => panic!("unexpected status {other}: {val}"),
        }
    }
    assert!(ok >= 1, "the request holding the slot must finish");
    assert!(
        busy >= 1,
        "requests that cannot start within queue_wait must get 503"
    );
}
