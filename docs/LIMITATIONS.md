# Known Limitations, Assumptions, and Residual Risks

FieldShift Optimizer is a transparent decision-support prototype. The limitations below are explicitly disclosed in the documentation and interface so users and judges do not over-interpret model outputs.

---

## 1. Decision-Support Boundaries & Prohibited Claims

- **Supports, Does Not Replace, the Farmer:** Outputs are comparative indicators under historical weather records, not agronomic prescriptions. Local field conditions, operational capacity, machinery availability, pest pressure, and market context remain the farmer's and agronomist's responsibility.
- **No Outcome Guarantees:** The tool does **not** predict or promise crop yield, farm profit, water savings, soil recovery, carbon sequestration, or pest suppression.
- **No Legal/Insurance ZARC Compliance Claim:** Although the model filters sowing windows using the official ZARC 2026/27 risk table, users must consult the official MAPA *Painel ZARC* for agricultural credit or insurance (PROAGRO/PSR) eligibility.
- **No Claim of Scientific Novelty or Method Superiority:** The system combines established multi-objective optimization, minimax regret, and public datasets. In our held-out retrospective test (`2010/11–2024/25`), minimax regret (`P2`) did **not** outperform expected-loss (`P1b`) or average-climate optimization (`P1`)—an explicitly reported **NEGATIVE** outcome.

---

## 2. Unverified Agronomic Hypotheses (`[N]` from `docs/AGRONOMY.md`)

The following parameters are explicit modeling hypotheses frozen for the MVP rather than site-measured constants:

1. **Soybean Cycle Duration:** Assumed `100 days` for Group I soybean (ZARC ordinances simulate `100 / 115 / 130 d` cycles; `100 d` is an inference).
2. **Crop-Stage Proportions:** Phenological phase fractions (`15% / 35% / 30% / 20%`) from 1st-season corn Group I are applied proportionally to soybean, 2nd-season corn, corn+brachiaria, and 2nd-season sorghum.
3. **Emergence & Turnaround Margin:** Fixed `7 days` from sowing to emergence (ZARC states `5–10 d`) and `0 days` margin between J1/J3 harvest and J2/J4 sowing.
4. **Legal Window Boundary Rule:** Decêndios only partially inside Goiás's legal soybean window (`dec1`) are excluded conservatively.
5. **Indicator Thresholds & Weights:**
   - Establishment failure threshold: `20 mm` in `10 days`.
   - Dry/rainy day threshold: `1 mm/day`.
   - Harvest rain threshold (`>= 6` rainy days in the final decêndio, documented for corn) is also applied to soybean and sorghum.
   - Equal weights across sub-indicators `e1–e5` within `g1`.
   - Fixed `15-day` brachiaria desiccation window prior to `J3`.
6. **Soil Input & Default:** No measured farm soil data is used. The user selects an `AD1–AD6` water-availability class and texture; the default `AD4 + clayey` and the correspondence between them are unverified hypotheses. No pedotransfer function (`IN SPA/MAPA 1/2022`) is implemented.
7. **Static 2026/27 ZARC Table:** The 2026/27 ZARC table is applied uniformly across all historical years (`1981–2025`).

---

## 3. Spatial, Temporal, and Data Scope Limits

- **Single NASA POWER Grid Cell:** Rio Verde is represented by a single grid cell `(-18.0, -50.625)` (inferred ~`0.5° × 0.625°` MERRA-2 resolution). Neighboring cells within the municipality show different annual rainfall totals (`1350 mm` vs. `1428 mm` in preliminary checks); the neighboring cell was not evaluated in this run.
- **Rainfed, 2-Year × 4-Window Horizon:** Irrigated systems, multi-year perennial transitions beyond 2 years, and crops outside the closed 5-crop + fallow Group I catalog are not modeled.
- **Excluded Climate Variables:** Solar radiation (`ALLSKY_SFC_SW_DWN`, unavailable for `1981–1983`) and potential evapotranspiration (`PET` returns HTTP 422 in POWER API) are excluded, so `g1` relies solely on precipitation and temperature (`PRECTOTCORR`, `T2M_MAX`, `T2M_MIN`), and `g2`/`g3` are calendar/species proxies rather than biophysical soil-water or carbon simulations.
- **Excluded NASA Datasets:** `IMERG`, `SMAP`, and `MODIS` require Earthdata credentials and are outside the MVP.

---

## 4. Residual Temporal Leakage (Declared, Cannot Be Eliminated)

- **ZARC Calibration Overlap:** Official ZARC 2026/27 risk tables were calibrated using historical climate stations that overlap the held-out test period (`2010/11–2024/25`). ZARC is applied identically as a fixed feasibility constraint to all compared policies (`P0`, `P1`, `P1b`, `P2`); its effect on the comparison was not quantified.
- **NASA POWER `PRECTOTCORR` Bias Correction:** NASA POWER's corrected precipitation product may incorporate long-term climatology overlapping the test period (`[N]`).

---

## 5. Evaluation, Statistical Power, and Protocol Deviations

- **Low Statistical Power:** The held-out test period (`2010/11–2024/25`, `14` two-year pairs) yields only `7` non-overlapping 2-pair blocks for the block bootstrap (`10,000` replicates, seed `20261114`) and `7` non-overlapping pairs for the Wilcoxon signed-rank test.
- **Dry-Year Threshold Skew:** `12` of the `14` test pairs fell below the construction dry-year threshold (`P25 Oct–Mar = 1098.9 mm`), so `M3` barely separates dry years from normal years in the test window.
- **Leave-One-Pair-Out (`M4c`) Simplification:** `M4c` keeps the full-construction `e3` medians fixed rather than recomputing them per leave-one-out fold (declared simplification).
- **Confidence Interval Choice (`Median` vs. `Mean`):** The frozen protocol (`v1.1`) specified "CI of `D`" without specifying median vs. mean. The evaluation code selected the **median** CI of `D` before the first run. For `P2` vs. `P1`, the median CI `[+0.0112, +0.0549]` is entirely above zero (**NEGATIVE**), whereas the mean CI `[-0.0234, +0.0466]` spans zero (which would be **INCONCLUSIVE**). Both CIs are reported in `reports/20261114T181200Z/eval_report.md`.
- **Protocol Transparency Note (First Run):** The frozen configuration hash (`067e569ec333fc7b7dcff85d925732f3746bf2f6525fd7f4d38e6935f64cbad1`) was written to `reports/FROZEN_CONFIG.json` on the first run (`20261114T181122Z`) rather than published in the coordination channel beforehand. Both runs (`20261114T181122Z` and `20261114T181200Z`) produced identical evaluation and `RunResult` hashes.
- **Snapshot Collection Deviation:** Collecting the NASA POWER snapshot required `4` API requests rather than a single request; the final offline snapshot was reconstructed from the saved raw JSON, with `snapshot_id` derived from its content SHA-256 (excluding the volatile `times` field).
- **Manifest Commit Traceability:** Both evaluation runs (`20261114T181122Z` and `20261114T181200Z`) executed on an uncommitted working tree; the manifest `commit` field points to the parent commit (`6661c45`). Re-running at `40bad6d` reproduces identical `RunResult` (`b4370332...fd66`) and evaluation (`504c28e8...4c18`) hashes.
- **Post-Test Exploratory Runs:** Interactive runs in the Streamlit app that alter soil class, objective weights, or baseline `P0` after inspection of the held-out test set are labeled `EXPLORATORY RUN` and do **not** replace or modify the frozen `NEGATIVE` evaluation.
- **Shared Indicator & Enumeration Code:** The evaluation module recomputes losses independently of the optimizer's selection, but reuses `indicators.loss_components` and `catalog.enumerate_plans` (and `catalog.verify_plan` does not check "fallow only when infeasible", which is covered by `config/zarc/VERIFICATION.md`). Thus, "independent evaluation" applies to policy selection and statistical testing, not to indicator definitions.
