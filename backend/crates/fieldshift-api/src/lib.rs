// AI-assisted (OpenBot agent)
pub mod engine;
use axum::{
    extract::{rejection::JsonRejection, DefaultBodyLimit, State},
    http::{HeaderValue, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use engine::{Engine, Inputs};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::{OnceCell, Semaphore};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

#[derive(Default)]
struct DefaultCache {
    run: OnceCell<Value>,
    evaluate: OnceCell<Value>,
    plans: OnceCell<Value>,
}

#[derive(Clone)]
struct AppState {
    engine: Arc<Engine>,
    slots: Arc<Semaphore>,
    queue_wait: Duration,
    default_cache: Arc<DefaultCache>,
}

/// Concurrency limits for heavy computations (`/v1/run`, `/v1/evaluate`, `/v1/plans`, `/v1/pareto`).
///
/// Requests wait in a FIFO queue (Tokio semaphore) for one of `slots` workers. A request that cannot
/// start within `queue_wait` gets `503 BUSY` with `Retry-After`, so a burst on a small instance
/// (Render Free: 0.1 CPU) fails fast instead of holding the connection until a proxy timeout.
#[derive(Clone, Copy, Debug)]
pub struct ApiLimits {
    pub slots: usize,
    pub queue_wait: Duration,
}

impl Default for ApiLimits {
    fn default() -> Self {
        Self {
            slots: 2,
            queue_wait: Duration::from_secs(QUEUE_WAIT_SECS),
        }
    }
}

/// Maximum time a computation request waits for a free slot before `503 BUSY`.
pub const QUEUE_WAIT_SECS: u64 = 30;
/// Seconds advertised in `Retry-After` when the wait for a slot runs out.
pub const RETRY_AFTER_SECS: u64 = 5;

pub fn resolve_bind_address(bind_address: Option<&str>, port: Option<&str>) -> String {
    if let Some(bind) = bind_address.map(str::trim).filter(|s| !s.is_empty()) {
        return bind.to_string();
    }
    if let Some(p) = port.map(str::trim).filter(|s| !s.is_empty()) {
        return format!("0.0.0.0:{p}");
    }
    "127.0.0.1:3000".to_string()
}
#[derive(Debug)]
struct ApiError(StatusCode, &'static str, String);
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let busy = self.1 == "BUSY";
        let mut resp = (
            self.0,
            Json(json!({"error":{"code":self.1,"message":self.2}})),
        )
            .into_response();
        if busy {
            resp.headers_mut().insert(
                axum::http::header::RETRY_AFTER,
                HeaderValue::from(RETRY_AFTER_SECS),
            );
        }
        resp
    }
}
fn input(value: Result<Json<Inputs>, JsonRejection>) -> Result<Inputs, ApiError> {
    let Json(i) = value.map_err(|e| ApiError(e.status(), "INVALID_JSON", e.body_text()))?;
    i.validate()
        .map_err(|e| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "INVALID_INPUT", e))?;
    Ok(i)
}
async fn execute_engine(
    s: AppState,
    i: Inputs,
    operation: &'static str,
) -> Result<Value, ApiError> {
    // Wait (FIFO) for a free slot instead of rejecting at once; give up after `queue_wait`.
    let permit = tokio::time::timeout(s.queue_wait, s.slots.clone().acquire_owned())
        .await
        .map_err(|_| {
            ApiError(
                StatusCode::SERVICE_UNAVAILABLE,
                "BUSY",
                format!(
                    "All computation slots stayed busy for {}s; retry after {RETRY_AFTER_SECS}s",
                    s.queue_wait.as_secs()
                ),
            )
        })?
        .map_err(|_| {
            ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL",
                "Computation semaphore closed".into(),
            )
        })?;
    let job = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        match operation {
            "evaluate" => s.engine.evaluate(&i),
            "plans" => s.engine.plans(&i),
            "pareto" => s.engine.run_value(&i).map(|v| {
                json!({"pareto":v["pareto"],
                "objectives":v["objectives"],"alternatives":v["alternatives"]})
            }),
            _ => s.engine.run_value(&i),
        }
    });
    tokio::time::timeout(Duration::from_secs(120), job)
        .await
        .map_err(|_| {
            ApiError(
                StatusCode::GATEWAY_TIMEOUT,
                "TIMEOUT",
                "Computation exceeded 120s".into(),
            )
        })?
        .map_err(|e| {
            tracing::error!(%e,"worker failed");
            ApiError(
                StatusCode::INTERNAL_SERVER_ERROR,
                "INTERNAL",
                "Computation worker failed".into(),
            )
        })?
        .map_err(|e| ApiError(StatusCode::UNPROCESSABLE_ENTITY, "COMPUTATION", e))
}

async fn calculate(
    s: AppState,
    i: Inputs,
    operation: &'static str,
) -> Result<Json<Value>, ApiError> {
    if s.engine.is_default_inputs(&i) {
        let val = match operation {
            "evaluate" => s
                .default_cache
                .evaluate
                .get_or_try_init(|| execute_engine(s.clone(), i, "evaluate"))
                .await?
                .clone(),
            "plans" => s
                .default_cache
                .plans
                .get_or_try_init(|| execute_engine(s.clone(), i, "plans"))
                .await?
                .clone(),
            "pareto" => {
                let v = s
                    .default_cache
                    .run
                    .get_or_try_init(|| execute_engine(s.clone(), i, "run"))
                    .await?;
                json!({
                    "pareto": v["pareto"],
                    "objectives": v["objectives"],
                    "alternatives": v["alternatives"]
                })
            }
            _ => s
                .default_cache
                .run
                .get_or_try_init(|| execute_engine(s.clone(), i, "run"))
                .await?
                .clone(),
        };
        return Ok(Json(val));
    }
    Ok(Json(execute_engine(s, i, operation).await?))
}
async fn health() -> Json<Value> {
    Json(json!({"status":"ok","language":"Rust","mode":"computed",
    "recomputation_available":true}))
}
async fn run(
    State(s): State<AppState>,
    v: Result<Json<Inputs>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    calculate(s, input(v)?, "run").await
}
async fn evaluate(
    State(s): State<AppState>,
    v: Result<Json<Inputs>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    calculate(s, input(v)?, "evaluate").await
}
async fn plans(
    State(s): State<AppState>,
    v: Result<Json<Inputs>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    calculate(s, input(v)?, "plans").await
}
async fn pareto(
    State(s): State<AppState>,
    v: Result<Json<Inputs>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    calculate(s, input(v)?, "pareto").await
}
async fn catalog(State(s): State<AppState>) -> Json<Value> {
    Json(s.engine.catalog_value())
}
async fn provenance(State(s): State<AppState>) -> Json<Value> {
    Json(json!({"manifest":s.engine.manifest,
    "quality":s.engine.quality,"protocol_sha256":s.engine.protocol_sha256,
    "source":"verified NASA POWER raw JSON; computed in Rust"}))
}
async fn meta(State(s): State<AppState>) -> Json<Value> {
    Json(
        json!({"version":env!("CARGO_PKG_VERSION"),"language":"Rust","mode":"computed",
        "protocol_version":"1.1","snapshot_id":s.engine.manifest["snapshot_id"],
        "config_hash":s.engine.default_config_hash,"protocol_sha256":s.engine.protocol_sha256}),
    )
}
async fn limitations() -> Json<Value> {
    Json(json!({"items":[
    "Decision support only: supports but does not replace the farmer or local agronomic advice.",
    "No claims of yield, profit, water savings, soil recovery or method superiority.",
    "[N] Crop cycles, emergence, phase lengths, rainfall/temperature thresholds and default AD4/clay soil are modeling assumptions.",
    "ZARC calibration overlaps test years; POWER rainfall correction may use a longer climatology.",
    "One regional climate cell; historical climate only; no measured farm soil; low statistical power.",
    "Construction 1981/82–2009/10; test 2010/11–2024/25. Scenario pairs overlap; 7 non-overlapping pairs enter Wilcoxon.",
    "Median CI was selected in code before the first run; the protocol did not specify mean vs median. Both are reported.",
    "Interactive changed-input runs after viewing the holdout are exploratory, not confirmatory.",
    "The evaluator independently selects and recomputes statistics but shares indicator definitions and enumeration.",
    "Four NASA POWER requests were made during collection. Frozen artifacts are preserved."]}))
}
async fn not_found() -> ApiError {
    ApiError(
        StatusCode::NOT_FOUND,
        "NOT_FOUND",
        "Unknown API route".into(),
    )
}
async fn wrong_method() -> ApiError {
    ApiError(
        StatusCode::METHOD_NOT_ALLOWED,
        "METHOD_NOT_ALLOWED",
        "Unsupported method".into(),
    )
}
async fn openapi() -> Json<Value> {
    Json(
        serde_json::from_str(include_str!("../openapi.json"))
            .expect("versioned OpenAPI is tested at build time"),
    )
}

pub fn router(engine: Engine, static_dir: Option<PathBuf>, origins: Vec<HeaderValue>) -> Router {
    router_with_engine(Arc::new(engine), static_dir, origins)
}

pub fn router_with_engine(
    engine: Arc<Engine>,
    static_dir: Option<PathBuf>,
    origins: Vec<HeaderValue>,
) -> Router {
    router_with_limits(engine, static_dir, origins, ApiLimits::default())
}

pub fn router_with_limits(
    engine: Arc<Engine>,
    static_dir: Option<PathBuf>,
    origins: Vec<HeaderValue>,
    limits: ApiLimits,
) -> Router {
    let state = AppState {
        engine,
        slots: Arc::new(Semaphore::new(limits.slots.max(1))),
        queue_wait: limits.queue_wait,
        default_cache: Arc::new(DefaultCache::default()),
    };
    let api = Router::new()
        .route("/health", get(health))
        .route("/meta", get(meta))
        .route("/catalog", get(catalog))
        .route("/plans", post(plans))
        .route("/run", post(run))
        .route("/evaluate", post(evaluate))
        .route("/pareto", post(pareto))
        .route("/provenance", get(provenance))
        .route("/limitations", get(limitations))
        .route("/openapi.json", get(openapi))
        .fallback(not_found)
        .method_not_allowed_fallback(wrong_method)
        .with_state(state);
    let mut app = Router::new().route("/health", get(health)).nest("/v1", api);
    if let Some(dir) = static_dir {
        let spa = ServeFile::new(dir.join("200.html"));
        app = app.fallback_service(ServeDir::new(dir).not_found_service(spa));
    } else {
        app = app.fallback(not_found);
    }
    app.layer(DefaultBodyLimit::max(64 * 1024))
        .layer(TraceLayer::new_for_http())
        .layer(
            CorsLayer::new()
                .allow_origin(origins)
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([axum::http::header::CONTENT_TYPE]),
        )
}
