# Known Limitations, Assumptions, and Residual Risks

FieldShift Optimizer is a transparent decision-support prototype. The limitations below are explicitly disclosed in the documentation and interface so users and judges do not over-interpret model outputs.

---

## 1. Decision-Support Boundaries & Prohibited Claims

- **Supports, Does Not Replace, the Farmer:** Outputs are comparative indicators under historical weather records, not agronomic prescriptions. Local field conditions, operational capacity, machinery availability, pest pressure, and market context remain the farmer's and agronomist's responsibility.
- **No Outcome Guarantees:** The tool does **not** predict or promise crop yield, farm profit, water savings, soil recovery, carbon sequestration, or pest suppression.
- **No Legal/Insurance ZARC Compliance Claim:** Although the model filters sowing windows using the official ZARC 2026/27 risk table, users must consult the official MAPA *Painel ZARC* for agricultural credit or insurance (PROAGRO/PSR) eligibility.
- **No Claim of Scientific Novelty or Universal Superiority:** The system combines established multi-objective optimization, minimax regret, and public datasets. We do not claim unprecedented scientific originality or that minimax regret outperforms other strategies in general.

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

- **Single NASA POWER Grid Cell:** Rio Verde is represented by a single grid cell `(-18.0, -50.625)` (inferred ~`0.5° × 0.625°` MERRA-2 resolution). Neighboring cells within the municipality show different annual rainfall totals (`1350 mm` vs. `1428 mm` in preliminary checks).
- **Rainfed, 2-Year × 4-Window Horizon:** Irrigated systems, multi-year perennial transitions beyond 2 years, and crops outside the closed 5-crop + fallow Group I catalog are not modeled.
- **Excluded Climate Variables:** Solar radiation (`ALLSKY_SFC_SW_DWN`, unavailable for `1981–1983`) and potential evapotranspiration (`PET` returns HTTP 422 in POWER API) are excluded, so `g1` relies solely on precipitation and temperature (`PRECTOTCORR`, `T2M_MAX`, `T2M_MIN`), and `g2`/`g3` are calendar/species proxies rather than biophysical soil-water or carbon simulations.
- **Excluded NASA Datasets:** `IMERG`, `SMAP`, and `MODIS` require Earthdata credentials and are outside the MVP.

---

## 4. Residual Temporal Leakage (Declared, Cannot Be Eliminated)

- **ZARC Calibration Overlap:** Official ZARC 2026/27 risk tables were calibrated using `>= 15` years of historical climate stations that overlap the held-out test period (`2010/11–2024/25`). Because ZARC is applied identically as a fixed feasibility constraint to all compared policies (`P0`, `P1`, `P1b`, `P2`), it does not favor one policy over another, but it remains a declared external overlap.
- **NASA POWER `PRECTOTCORR` Bias Correction:** NASA POWER's corrected precipitation product may incorporate long-term climatology overlapping the test period (`[N]`).

---

## 5. Evaluation & Statistical Power Limits

- `[RESULT PENDING]` (To be completed from `reports/` after the final integrated evaluation run, including exact test-period sample size, dry-year distribution relative to construction `P25`, block-bootstrap power with 7 non-overlapping blocks, and observed negative/inconclusive/positive outcomes.)
