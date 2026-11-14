// AI-assisted (OpenBot agent)
use anyhow::{Context, Result};
use axum::http::HeaderValue;
use fieldshift_api::{engine::Engine, router};
use std::{env, path::PathBuf};
#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "fieldshift_api=info,tower_http=info".into()),
        )
        .init();
    let root = env::var_os("FIELDSHIFT_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.."));
    let engine = Engine::load(&root).context("Cannot verify scientific inputs")?;
    let origins = env::var("FRONTEND_ORIGIN")
        .unwrap_or_else(|_| "http://localhost:5173".into())
        .split(',')
        .map(|s| s.trim().parse::<HeaderValue>())
        .collect::<Result<Vec<_>, _>>()
        .context("Invalid FRONTEND_ORIGIN")?;
    let static_dir = env::var_os("FIELDSHIFT_STATIC_DIR").map(PathBuf::from);
    let bind = env::var("BIND_ADDRESS").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind,"Serving computed Rust scientific results");
    axum::serve(listener, router(engine, static_dir, origins)).await?;
    Ok(())
}
