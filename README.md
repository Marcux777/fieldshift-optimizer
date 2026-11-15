# FieldShift Optimizer

**Explore and Compare Crop Rotation Trade-offs Under Climate Uncertainty**

Project developed for the [NASA Space Apps Challenge 2026 — Field Shift: Adapting Farms with NASA Data](https://www.spaceappschallenge.org/2026/challenges/adapting-farms-with-nasa-data/).

> **Decision-Support Notice:** FieldShift Optimizer **supports—and does not replace—the farmer's judgment** or local agronomic assistance. It does **not** promise or guarantee crop yield, profit, water savings, or soil recovery, and makes **no claim of unprecedented scientific novelty** or method superiority.

---

## Status & Key Outcome

- **Live app:** https://fieldshift-optimizer.onrender.com (Rust API + SvelteKit, Render Free; the free instance sleeps after 15 minutes without traffic, so the first request can take about a minute).
- **Python reference implementation (oracle for differential tests, verified in container `fieldshift-dev`):** `170 passed in 17.53s` (`pytest -q`); `ruff check .` passed (`All checks passed!`).
- **Retrospective Evaluation Outcome (`2010/11–2024/25` held-out test):** **NEGATIVE.** On the frozen protocol (`v1.1`, `|X| = 576` feasible plans, weights `0.50 / 0.25 / 0.25`), minimax regret (`P2`) did **not** outperform expected-loss (`P1b`) or average-climate optimization (`P1`):
  - **`P2` vs. `P1b` (primary):** **NEGATIVE** — test max regret `0.1744` (`P2`) vs. `0.1466` (`P1b`), median paired difference `D = +0.0064`, 95% CI `[-0.0609, +0.0375]`, Wilcoxon `p = 0.6875` (`7` non-overlapping pairs).
  - **`P2` vs. `P1` (secondary):** **NEGATIVE** — `P2` lowers test max regret (`0.1744` vs. `0.2811`) but performs worse in most test years (`2` wins, `4` ties, `8` losses; median `D = +0.0228`, 95% CI `[+0.0112, +0.0549]`). Using the **mean** CI (`[-0.0234, +0.0466]`) instead of the median would classify this comparison as **INCONCLUSIVE**.
  - **`P0` (current baseline):** Viable under ZARC (`<= 20%`), but worst on the test set (max regret `0.3196`, `0` years within `ε = 0.02`).
  - **15-weight sensitivity grid (`P2` vs. `P1b`):** **NEGATIVE** in `8` points and **INCONCLUSIVE** in `7` points (`0` sustained).
- **What the App Delivers:** Rather than claiming minimax regret "wins," FieldShift Optimizer provides a transparent, reproducible tool to **compare** rotation strategies side by side, **explore** the Pareto frontier (`36` distinct non-dominated objective vectors; `50` plans including `14` vector ties; `6` distinct crop-signature alternatives), inspect the price of robustness in average climate (`M5 = 0.0080`), and see honestly when robustness did **not** pay off out-of-sample.
- **Public Repository:** https://github.com/Marcux777/fieldshift-optimizer (public, MIT)
- **Live Deployment URL:** https://fieldshift-optimizer.onrender.com

---

## MVP Scope

- **Region & System:** Rio Verde, Goiás, Brazil (IBGE `5218805`), rainfed agriculture, 2-agricultural-year horizon (4 cropping windows: `J1`–`J4`, Sept 1 to Aug 31).
- **NASA Climate Data:** Versioned offline snapshot from NASA POWER Daily Point API (`v2.10.3`, `MERRA2` + `POWER`, cell `-18.0, -50.625`, `1981-01-01` to `2025-12-31`, `PRECTOTCORR`, `T2M`, `T2M_MAX`, `T2M_MIN`).
- **Agronomic Constraints:** Official ZARC 2026/27 risk-zoning table for Rio Verde (fixed `<= 20%` risk threshold), Group I cycle per crop (soybean, 1st-season corn, 2nd-season corn, 2nd-season corn + brachiaria, 2nd-season grain sorghum, and fallow only when no 2nd crop is feasible), and Goiás soybean legal sowing/sanitary-void rules (`IN Agrodefesa 6/2024`).
- **Multi-Objective Optimization & Pareto Frontier:** Three objectives in `[0, 1]` (climate exposure `g1`, vegetation cover discontinuity `g2`, crop diversity deficit `g3`), default weights `0.50 / 0.25 / 0.25` plus a 15-point sensitivity grid.
- **Policy Comparison:** Side-by-side comparison of `P0` (current plan), `P1` (average-climate optimization), `P1b` (expected-loss optimization), and `P2` (minimax regret).
- **Retrospective Evaluation:** Strict temporal split (`1981/82–2009/10` construction, 28 two-year pairs; `2010/11–2024/25` held-out test, 14 two-year pairs) with independent loss recomputation and leakage checks.
- **Interface:** SvelteKit 5 static web app served by the Rust API (`fieldshift-api`), showing sources, assumptions, trade-offs, and limitations. The original Streamlit app (`app.py`) stays in the repository only as the reference implementation.

---

---

## Rust + SvelteKit 5 Migration (`feat/rust-svelte-migration`)

This branch adds a complete pure-Rust scientific backend (`backend/`, 6-crate Cargo workspace) and a **SvelteKit 5 + TypeScript + Tailwind CSS v4** static frontend (`frontend/`), verified bit-for-bit against the Python baseline (`16cc391318b57fbf44a31c75c994fbb0683b3115`):

- **Rust Backend (`backend/`):** `fieldshift-numeric`, `fieldshift-io`, `fieldshift-model`, `fieldshift-eval`, `fieldshift-cli`, `fieldshift-api` (`176 passed; 0 failed` across unit, ported 1-to-1, integration, and differential suites; `cargo clippy --locked --workspace --all-targets -- -D warnings` clean).
- **Frontend (`frontend/`):** SvelteKit 5 + TypeScript + Tailwind CSS v4 (`svelte-check` `0 errors, 0 warnings`; Vitest `5 passed`; Playwright E2E `6 passed` on `desktop-chromium` `1280×800` and `mobile-chromium` `390×844`).
- **Bit-for-Bit Parity Verified:**
  - Canonical `RunResult` SHA-256: `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66`
  - Canonical `EvalReport` SHA-256: `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18`
  - Primary retrospective outcome (`P2` vs `P1b`): **`NEGATIVO`** (`0.1744` vs `0.1466`, `D = +0.0064`, 95% CI `[-0.0609, +0.0375]`, Wilcoxon `p = 0.6875`)
- **Documentation:** See [`docs/RUN.md`](docs/RUN.md), [`docs/RUST_MIGRATION.md`](docs/RUST_MIGRATION.md), [`docs/MIGRATION_REPORT.md`](docs/MIGRATION_REPORT.md), [`docs/RUST_CONTRACT.md`](docs/RUST_CONTRACT.md), and [`docs/HOSTING.md`](docs/HOSTING.md).

## Running Locally

**Rust + SvelteKit app (what is deployed):** see [`docs/RUN.md`](docs/RUN.md) for the full commands (backend tests, offline CLI reproduction, frontend checks and Playwright, single-container Docker build). In short:

```bash
cd frontend && npm ci && npm run build && cd ..
FIELDSHIFT_ROOT="$PWD" FIELDSHIFT_STATIC_DIR="$PWD/frontend/build" BIND_ADDRESS=127.0.0.1:3000 \
  cargo run --locked --release --manifest-path backend/Cargo.toml -p fieldshift-api
```

The Rust toolchain is pinned in [`rust-toolchain.toml`](rust-toolchain.toml) (same version in CI and `backend/Dockerfile`).

**Python reference implementation (oracle for the differential tests):** runs inside a single reusable container (`fieldshift-dev`) without installing Python dependencies on the host.

1. Start the container:
   ```bash
   export DOCKER_SHARED_CACHE_ROOT=$HOME/.cache/docker-shared
   docker compose up -d dev
   ```

2. Run the test suite and linter inside the container:
   ```bash
   docker compose exec -T dev pytest -q
   docker compose exec -T dev ruff check .
   ```

3. Optionally run the legacy Streamlit app (reference only, not deployed):
   ```bash
   docker compose exec dev streamlit run app.py --server.port 8501 --server.address 0.0.0.0
   ```
   Then open `http://127.0.0.1:8501` in your browser.

---

## Deployment (Render Free, single container)

The public app is one Docker image built from `backend/Dockerfile` (repository root as build context): the Rust API, the SvelteKit static build, `config/` and the verified NASA POWER snapshot. It contains no Python and does not replay frozen reports; every result is computed in Rust. Details and limits of the free plan are in [`docs/HOSTING.md`](docs/HOSTING.md).

- **Public App URL:** https://fieldshift-optimizer.onrender.com

---

## Repository Structure

```text
fieldshift-optimizer/
├── app.py                          # Legacy Streamlit entrypoint (reference implementation, not deployed)
├── backend/                        # Rust workspace: numeric, io, model, eval, cli, api (+ Dockerfile for deployment)
├── frontend/                       # SvelteKit 5 + TypeScript static frontend (Vitest + Playwright)
├── rust-toolchain.toml             # Pinned Rust toolchain (local, CI and container)
├── compose.yaml                    # Dev/test container for the Python reference implementation
├── Dockerfile                      # Python 3.12 + uv container image
├── pyproject.toml                  # Project metadata and dependencies
├── uv.lock                         # Locked dependency versions
├── LICENSE                         # MIT License
├── config/
│   ├── catalog.toml                # Crops, windows, and ZARC filter rules
│   ├── indicators.toml             # Climate exposure, cover, and diversity formulas
│   ├── soil.toml                   # Soil AD classes and textures (default hypothesis: AD4 + clayey)
│   └── zarc/                       # Official ZARC 2026/27 Rio Verde slice + SHA256SUMS
├── data/
│   └── snapshots/                  # Versioned NASA POWER climate snapshot + manifest.json
├── docs/
│   ├── AGRONOMY.md                 # Agronomic rules and [V]/[N] assumptions
│   ├── AI_USE.md                   # Synthetic Intelligence (SI) disclosure
│   ├── CONTRACT.md                 # Module interfaces and ownership
│   ├── DATA_SOURCES.md             # Data provenance and citations
│   ├── DECISIONS.md                # Frozen technical scope decisions
│   ├── EVALUATION_PROTOCOL.md      # Frozen retrospective evaluation protocol (v1.1)
│   ├── HACKATHON_RULES.md          # Verified NASA Space Apps 2026 rules
│   ├── LIMITATIONS.md              # Known limitations, hypotheses, and residual leakage
│   ├── REFERENCES.md               # Complete verifiable reference list
│   └── SUBMISSION.md               # Project Page draft, demo scripts, and checklist
├── reports/                        # Evaluation run outputs and frozen config hashes
├── scripts/
│   ├── collect_snapshot.py         # Snapshot collector (only script using network)
│   └── run_evaluation.py           # Independent retrospective evaluation runner
├── src/fieldshift/
│   ├── catalog.py                  # Feasible rotation enumeration and verification
│   ├── data.py                     # Snapshot loader and manifest/hash verification
│   ├── evaluation.py               # Out-of-sample metrics, bootstrap, and Wilcoxon tests
│   ├── indicators.py               # Pure climate, cover, and diversity loss functions
│   ├── optimization.py             # Scenarios, Pareto frontier, P0/P1/P1b/P2 selection
│   ├── pipeline.py                 # End-to-end orchestration for the UI
│   └── ui_app.py                   # Legacy Streamlit interface (reference only)
└── tests/                          # Unit, leakage, pipeline, and AppTest suites
```

---

## Verification & Reproducibility Hashes

- **Test Suite (`docker compose exec -T dev pytest -q`):** `170 passed in 17.53s`
- **Linter (`docker compose exec -T dev ruff check .`):** `All checks passed!`
- **Snapshot ID:** `power-daily-2.10.3-lat-18_lon-50.625-46c276253b44` (`manifest.json` SHA-256: `2de4764ad0c1a25f85d3989488d175236171db89505705c8c9f3f69f50f2d76b`)
- **Protocol v1.1 SHA-256:** `ee8e51c5b2eecc7319818a5645ef15b84d677eabab35caad1a7e9fc41deaefb9`
- **Evaluation Result Hash (`reports/20261114T181122Z/` & `20261114T181200Z/`):** `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18`
- **RunResult Hash:** `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66`

---

## License

Code in this repository is licensed under the **MIT License** ([`LICENSE`](LICENSE)). External datasets (NASA POWER, MAPA ZARC, Embrapa, Agrodefesa) remain subject to their respective provider terms and attribution requirements documented in [`docs/REFERENCES.md`](docs/REFERENCES.md).
