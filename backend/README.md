# FieldShift — Rust migration backend (INCOMPLETE)

The Rust service currently **only reads and serves verified, frozen reports** from the Python baseline. It does not recompute crop plans, run NASA POWER ingestion, enumerate ZARC-feasible plans, evaluate indicators, or reproduce the original optimization. Do not advertise full backend parity. Missing operations must fail explicitly.

## Local
From the repository root:

```bash
cargo test --manifest-path backend/Cargo.toml
cargo run --manifest-path backend/Cargo.toml
curl -fsS http://localhost:3000/api/health
```

The server checks `reports/20261114T181122Z/SHA256SUMS` before startup and refuses inconsistent/altered data.

Read-only endpoints: `/api/health`, `/api/v1/summary`, `/api/v1/report`, `/api/v1/run`, `/api/v1/policies`, `/api/v1/provenance`.

Environment: `FIELDSHIFT_ROOT` (root with reports), `FRONTEND_ORIGIN` (exact browser origin, default localhost:5173), `BIND_ADDRESS` (default 0.0.0.0:3000).

## Full migration gates
1. Port snapshot validation and immutable offline NASA data handling; verify SHA-256 and unit contracts.
2. Port catalog, 20% ZARC, enumeration and independent feasibility verifier.
3. Port indicators, construction scenarios, loss tensors, Pareto and all policies.
4. Port independent retrospective evaluation, bootstrap and Wilcoxon checks.
5. Compare Rust output against pinned JSON hashes, plan IDs, component matrices and numerical tolerances; retain regression cases M-1/M-2.
6. Only after parity and live browser tests should the Python/Streamlit implementation be removed.

No benchmarks/performance claims have been established; compiled tests are not evidence of scientific parity.
