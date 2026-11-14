<!-- AI-assisted (OpenBot agent) -->
# Running and Verifying FieldShift Optimizer (Rust + SvelteKit 5)

All commands below run offline using the committed, SHA-256-verified NASA POWER climate snapshot (`data/snapshots/power-daily-2.10.3-lat-18_lon-50.625-46c276253b44`) and ZARC 2026/27 tables (`config/`).

## 1. Backend Formatting, Linting, and Full Test Suite

From `backend/`:

```bash
cd backend
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
```

## 2. Offline CLI Reproduction and Differential Verification

Run the complete optimization and independent retrospective evaluation from the repository root:

```bash
cargo run --locked --release --manifest-path backend/Cargo.toml -p fieldshift-cli -- \
  run-evaluation --output artifacts/rust/reproduction
```

Compare generated outputs against the frozen Python reference reports (`SERIALIZATION_ONLY` means identical numeric values and canonical SHA-256 hashes, differing only in whitespace formatting):

```bash
cargo run --locked --release --manifest-path backend/Cargo.toml -p fieldshift-cli -- \
  diff-reference artifacts/rust/reproduction/run_result.json fixtures/reference/golden_run_result.json
cargo run --locked --release --manifest-path backend/Cargo.toml -p fieldshift-cli -- \
  diff-reference artifacts/rust/reproduction/eval_report.json fixtures/reference/golden_eval_report.json
```

Verify the climate snapshot integrity directly:

```bash
cargo run --locked --release --manifest-path backend/Cargo.toml -p fieldshift-cli -- \
  verify-snapshot data/snapshots/power-daily-2.10.3-lat-18_lon-50.625-46c276253b44
```

## 3. Frontend Checks, Build, and Playwright E2E Suite

From `frontend/`:

```bash
cd frontend
npm run gen:api
npm run check
npm run test -- --run
npm run build
npm run test:e2e
```

## 4. Running the Integrated Web Application Locally

Build the static frontend and launch `fieldshift-api`:

```bash
cd frontend && npm run build && cd ..
FIELDSHIFT_ROOT="$PWD" \
FIELDSHIFT_STATIC_DIR="$PWD/frontend/build" \
BIND_ADDRESS="127.0.0.1:3000" \
cargo run --locked --release --manifest-path backend/Cargo.toml -p fieldshift-api
```

Open `http://127.0.0.1:3000` in your browser (or inspect `http://127.0.0.1:3000/v1/health` and `http://127.0.0.1:3000/v1/openapi.json`).

## 5. Single-Container Docker Build and Smoke Test

```bash
docker build -f backend/Dockerfile -t fieldshift:rust-migration .
docker run --name fieldshift-rust -d --read-only --tmpfs /tmp:rw,noexec,nosuid,size=32m \
  -p 127.0.0.1:3000:3000 fieldshift:rust-migration
# Reuse with: docker start fieldshift-rust / docker stop fieldshift-rust
```
