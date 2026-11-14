# Rust + SvelteKit migration (authorized 14/11/2026)

## Decision and baseline
User requested replacement of all Python backend code with Rust and a better frontend. This branch is an **experimental incremental port**, not a completed replacement. Base: `16cc391318b57fbf44a31c75c994fbb0683b3115`, 170 Python tests reported passing, historical results frozen with outcome **NEGATIVO**. Do not change `docs/EVALUATION_PROTOCOL.md` (v1.1) or evidence already submitted. Preserve `main` until independent parity.

## Architecture
- Rust `Axum` API (`backend/`), computational core should be pure Rust (separate from I/O), no hidden Python or FFI.
- SvelteKit 5 / TypeScript static frontend (`frontend/`), responsive and accessible, connecting to Rust JSON API.
- Initial end-to-end slice: read SHA-256 checked frozen `reports/20261114T181122Z/`, present scientific outcome and policy/pareto/sensitivity data. All endpoints read-only.
- Deployment requires Rust-capable hosting (container) plus static frontend hosting. Streamlit Cloud is **not** a deployment option for this target stack.

## Current status and missing functionality
Implemented in this branch: Rust server with frozen artifact validation, read-only API endpoints, first standalone deterministic loss/regret/selection primitives, unit tests; SvelteKit UI with overview, strategy comparisons, Pareto visualization, data provenance, mobile styles; separate experimental CI.

**Not implemented**: end-to-end NASA snapshot ingestion/quality checks, ZARC filter and plan enumeration, crop calendar constraints, indicator loss components, climate scenario preparation, Pareto computation, 15-weight reoptimization, independent test evaluation, bootstrap/Wilcoxon, recomputed report hashes, working live deployment. The original Python remains deliberately intact, and no parity claims are made.

## Migration acceptance gate
1. Rust parser verifies NASA POWER `daily.parquet` (or equivalent conversion from the immutable raw snapshot), fill-value handling, date coverage and SHA-256; zero synthetic fallback.
2. Rust enumerates exactly **576** valid default plans, same IDs/order and independent feasibility decisions with risk <=20%; M-1 and M-2 mutations fail as before.
3. Rust computes all policy choices `P0/P1/P1b/P2` for every one of 15 weight settings with identical deterministic ties; scenarios remain split (28 construction, 14 held-out test) and no leakage.
4. Independent Rust evaluator reproduces every numeric result within a documented, agreed tolerance, plus classification, confidence intervals, seeds and exclusion logs. Floating-point exact-hash parity requires canonical JSON and matching numeric operations, not a guess.
5. Run fixture-based differential testing against frozen outputs, mutation tests, integration/API tests, browser/mobile checks, `cargo fmt`, `clippy`, unit/integration tests, frontend build. Do not claim '170 passed' for Rust: those belong to original Python.
6. After all gates pass, migrate docs/deploy and remove Python, `uv.lock`, Streamlit and obsolete Docker wiring in a separate reviewed change. Update evidence only with transparent versioning and without revising frozen protocol.

## Risks/stop conditions
High risk near NASA Space Apps submission deadline: loss of scientific equivalence and changed hosting requirements. Stop replacing the production app if any parity check fails or the new deployment is unavailable. Existing Python `main` remains the stable and audited version. No deployment or submission is authorized by this migration request.
