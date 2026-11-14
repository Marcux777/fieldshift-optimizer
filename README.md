# FieldShift Optimizer

**Robust Crop Rotation Decisions Under Climate Uncertainty**

Project developed for the [NASA Space Apps Challenge 2026 — Field Shift: Adapting Farms with NASA Data](https://www.spaceappschallenge.org/2026/challenges/adapting-farms-with-nasa-data/).

> **Decision-Support Notice:** FieldShift Optimizer **supports—and does not replace—the farmer's judgment** or local agronomic assistance. It does **not** promise or guarantee crop yield, profit, water savings, or soil recovery, and makes **no claim of unprecedented scientific novelty**.

---

## Status

- **Data & Core Modules:** Implemented (`src/fieldshift/`), using a versioned offline NASA POWER snapshot (`power-daily-2.10.3-lat-18_lon-50.625-46c276253b44`).
- **Tests & Evaluation Results:** `[RESULT PENDING]` (awaiting final integrated run, test suite verification, and evaluation report).
- **Live Deployment URL:** `[RESULT PENDING]`

---

## MVP Scope

- **Region & System:** Rio Verde, Goiás, Brazil (IBGE `5218805`), rainfed agriculture, 2-agricultural-year horizon (4 cropping windows: `J1`–`J4`, Sept 1 to Aug 31).
- **NASA Climate Data:** Versioned offline snapshot from NASA POWER Daily Point API (`v2.10.3`, `MERRA2` + `POWER`, cell `-18.0, -50.625`, `1981-01-01` to `2025-12-31`, `PRECTOTCORR`, `T2M`, `T2M_MAX`, `T2M_MIN`).
- **Agronomic Constraints:** Official ZARC 2026/27 risk-zoning table for Rio Verde (fixed `<= 20%` risk threshold), Group I cycle per crop (soybean, 1st-season corn, 2nd-season corn, 2nd-season corn + brachiaria, 2nd-season grain sorghum, and fallow only when no 2nd crop is feasible), and Goiás soybean legal sowing/sanitary-void rules (`IN Agrodefesa 6/2024`).
- **Multi-Objective Optimization & Pareto Frontier:** Three objectives in `[0, 1]` (climate exposure `g1`, vegetation cover discontinuity `g2`, crop diversity deficit `g3`), default weights `0.50 / 0.25 / 0.25` plus a 15-point sensitivity grid.
- **Policy Comparison:** Side-by-side comparison of:
  - `P0`: Current baseline plan (user-configurable; default hypothesis: soybean in `dec29` + 2nd-season corn)
  - `P1`: Conventional optimization under average construction climate
  - `P1b`: Expected-loss optimization across construction scenarios
  - `P2`: Minimax regret optimization across construction scenarios
- **Retrospective Evaluation:** Strict temporal split (`1981/82–2009/10` construction, 28 two-year pairs; `2010/11–2024/25` held-out test, 14 two-year pairs) with independent loss recomputation and leakage checks.
- **Interface:** Streamlit web application displaying sources, assumptions, trade-offs, and limitations.

---

## Running Locally with Docker Compose

Development and testing run inside a single reusable container (`fieldshift-dev`) without installing Python dependencies on the host.

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

3. Run the Streamlit app locally (offline, using the versioned snapshot):
   ```bash
   docker compose exec dev streamlit run app.py --server.port 8501 --server.address 0.0.0.0
   ```
   Then open `http://127.0.0.1:8501` in your browser.

---

## Deployment on Streamlit Community Cloud

Streamlit Community Cloud does **not** use Docker. It installs Python dependencies directly from the repository (`pyproject.toml` / `uv.lock`) and launches `app.py` at the repository root. All runtime computations operate offline using the committed snapshot in `data/snapshots/` and configuration files in `config/`.

- **Public App URL:** `[RESULT PENDING]`

---

## Repository Structure

```text
fieldshift-optimizer/
├── app.py                          # Streamlit Cloud entrypoint
├── compose.yaml                    # Reusable dev/test container definition
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
│   └── ui_app.py                   # Streamlit interface components
└── tests/                          # Unit, leakage, pipeline, and AppTest suites
```

---

## Verification & Results

- **Test Suite (`pytest -q`):** `[RESULT PENDING]`
- **Evaluation Outcome (`P2` vs. `P1b` / `P1` / `P0`):** `[RESULT PENDING]`
- **Result & RunResult Hashes:** `[RESULT PENDING]`

---

## License

Code in this repository is licensed under the **MIT License** ([`LICENSE`](LICENSE)). External datasets (NASA POWER, MAPA ZARC, Embrapa, Agrodefesa) remain subject to their respective provider terms and attribution requirements documented in [`docs/REFERENCES.md`](docs/REFERENCES.md).
