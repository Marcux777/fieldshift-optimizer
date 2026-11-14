<!-- AI-assisted (OpenBot agent) -->
# Rust + SvelteKit 5 Migration (`feat/rust-svelte-migration`)

## Status: COMPLETED (E0–E9 verified bit-for-bit against Python baseline `16cc391`)

This branch implements the full migration of **FieldShift Optimizer** from Python/Streamlit to a pure-Rust scientific backend (`backend/`, 6-crate Cargo workspace) and a **SvelteKit 5 + TypeScript + Tailwind CSS v4** static frontend (`frontend/`), preserving scientific equivalence bit-for-bit with the audited Python baseline (`16cc391318b57fbf44a31c75c994fbb0683b3115`) and frozen evaluation protocol **v1.1** (`docs/EVALUATION_PROTOCOL.md`, SHA-256 `ee8e51c5b2eecc7319818a5645ef15b84d677eabab35caad1a7e9fc41deaefb9`).

- **Primary Retrospective Outcome Preserved:** `NEGATIVO` (`P2` vs `P1b` test max regret `0.1744` vs `0.1466`, median paired difference `D = +0.0064`, 95% block-bootstrap CI `[-0.0609, +0.0375]`, exact Wilcoxon `p = 0.6875`).
- **Canonical `RunResult` SHA-256:** `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66` (identical bit-for-bit to Python).
- **Canonical `EvalReport` SHA-256:** `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18` (identical bit-for-bit to Python).
- **Zero Python in Runtime:** All optimization, indicator computation, scenario splitting, block bootstrap (`PCG64` + `SeedSequence`), exact Wilcoxon, and API responses are computed natively in Rust without reading `reports/` or `fixtures/` at runtime.
- **Frozen Artifacts Untouched:** `main`, `docs/EVALUATION_PROTOCOL.md`, `reports/**`, `config/**`, and `data/snapshots/**` have zero modifications.

## Workspace Architecture (`backend/`)

| Crate | Role | Key Verification |
|---|---|---|
| `fieldshift-numeric` | Shewchuk exact `fsum`, `py_round`, `np_round`, `np_quantile`, `np_median`, pairwise `np_sum`, NumPy `SeedSequence` + `Pcg64` + Lemire bounded integers, canonical JSON (`py_float_repr`), agricultural `Date` calendar | 4 differential suites: 1,030,009 RNG draws, 70,000 bootstrap indices (`seed=20261114`), 16,436 dates, canonical JSON vectors |
| `fieldshift-io` | Verified NASA POWER snapshot reader (`raw/power_daily.json.gz`, `manifest.json`, `SHA256SUMS`, `-999.0 -> NaN`, strict duplicate-key rejection, path-traversal guard), schema-2 snapshot writer, TOML & ZARC CSV loaders | 10 test groups (26+ adverse contracts) + 65,744 climate cells bit-for-bit |
| `fieldshift-model` | 365-day agricultural calendar, ZARC `<= 20%` enumeration (`verify_plan`, `diagnose_infeasible`), `g1/g2/g3` indicators, temporal split (`28` construction / `14` test pairs), lexicographic `P0/P1/P1b/P2` selection across 15 weights, Pareto frontier (`36` vectors / `50` plans / `6` alternatives) | 126 tests (103 ported 1-to-1 from Python + 8 differential suites + 15 unit tests; 74,304 indicator components with `max |Δ| = 0.0`) |
| `fieldshift-eval` | Independent retrospective evaluator (`evaluate_plans`), `IndicatorLossModel` adapter with construction-only `e3` climatology and cache invalidation, block bootstrap, exact $2^n$ Wilcoxon, Markdown renderer | 30 tests (including M-1 and M-2 negative mutation controls verified to fail when triggered) |
| `fieldshift-cli` | `run-evaluation`, `diff-reference` (`IDENTICAL` / `SERIALIZATION_ONLY` / `SCIENTIFIC`), `collect-snapshot` (offline-first), `verify-snapshot` | End-to-end evaluation in ~3.5s, ~23 MiB max RSS, exact hashes |
| `fieldshift-api` | Axum 0.8 HTTP server (`/health`, `/v1/health`, `/v1/meta`, `/v1/catalog`, `/v1/provenance`, `/v1/limitations`, `/v1/openapi.json`, `POST /v1/plans`, `POST /v1/run`, `POST /v1/evaluate`, `POST /v1/pareto`) + static SPA hosting | 5 integration/differential suites against golden fixtures and boundary contracts |

## Frontend Architecture (`frontend/`)

- **Stack:** SvelteKit 5 (`@sveltejs/adapter-static`) + TypeScript + Tailwind CSS v4.
- **Typed Contract:** OpenAPI 3.1 (`backend/crates/fieldshift-api/openapi.json`) -> generated TypeScript types (`src/lib/api/openapi.gen.ts`) + validated boundary client (`src/lib/api/client.ts`).
- **Verification:** `npm run check` (`0 errors, 0 warnings`), `vitest --run` (`5 passed`), `playwright test` (`6 passed` across `desktop-chromium` `1280×800` and `mobile-chromium` `390×844` against the live Rust binary).

## Remaining Gated Stages (Require Explicit User Authorization)

- **E10 (Public Deployment):** Single-container `backend/Dockerfile` (`fieldshift:rust-migration`) and free hosting analysis (`docs/HOSTING.md`) are ready and locally smoke-tested (`--read-only`, `uid=65534`, no Python). Public deployment awaits explicit user authorization.
- **E11 (Legacy Python/Streamlit Removal):** Python source (`src/fieldshift/`), `app.py`, and `uv.lock` remain intact until a separate user-authorized cleanup PR after final review.
