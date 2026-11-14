# NASA Space Apps 2026 — Submission Draft & Checklist

> **Target:** [Field Shift: Adapting Farms with NASA Data](https://www.spaceappschallenge.org/2026/challenges/adapting-farms-with-nasa-data/)
> **Internal Deadline:** 2026-11-15 21:00 BRT (Official Deadline: 2026-11-15 23:59 in Team Owner's timezone).
> **Note:** Do **not** submit externally without explicit user approval. Remaining `[RESULT PENDING]` slots are strictly for public URLs, team member details, SI media confirmation, and UI screenshot.

---

## 1. Project Page Form Fields (English Draft)

### Project Name
```text
FieldShift Optimizer — Explore and Compare Crop Rotation Trade-offs Under Climate Uncertainty
```

### High-Level Summary
```text
FieldShift Optimizer is an open-source decision-support web application to explore and compare rainfed crop rotation strategies in Rio Verde, Goiás, Brazil, over a two-year (four-window) horizon. Using a versioned historical climate snapshot from NASA POWER alongside official Brazilian ZARC sowing-risk tables, it compares a farmer's current rotation against average-climate optimization, expected-loss optimization, and minimax regret across climate exposure, cover continuity, and crop diversity. The tool supports—and does not replace—farmer judgment, transparently showing Pareto trade-offs, the cost of robustness in average climate, and honest out-of-sample results (including when minimax regret does not pay off), without promising yield, profit, water, or soil outcomes.
```

### Link to Demo
```text
URL: [RESULT PENDING: public URL without login to <= 7-slide deck OR <= 30-second video]
Format: [RESULT PENDING: 7-slide deck / 30-second video]
```

### Link to Final Project
```text
Live App URL: [RESULT PENDING: public Streamlit Community Cloud URL without login]
Public Repository URL: [RESULT PENDING: public GitHub URL https://github.com/Marcux777/fieldshift-optimizer verified without login]
```

### Detailed Description
```markdown
### Problem & Central Question
Rainfed farmers in Rio Verde (Goiás, Brazil) choose multi-season crop rotations under substantial rainfall and dry-spell uncertainty. A rotation that looks optimal under average historical weather can suffer high climate exposure during dry years, while a worst-case-focused plan may sacrifice performance in typical years. FieldShift Optimizer helps explore a practical question: *"How do different crop rotation strategies compare when we do not know which climate conditions will occur?"*

### What the Application Delivers
FieldShift Optimizer is a Streamlit decision-support tool for a two-agricultural-year, four-window (`J1`–`J4`) rainfed planning horizon. Users specify soil parameters (`AD1–AD6` water-availability class and texture), baseline rotation (`P0`), and objective preferences. The engine enumerates all `576` agronomically feasible rotations (at default `AD4 + clayey` soil) under official ZARC 2026/27 (`<= 20%` risk) and Goiás legal sowing/sanitary-void rules (`IN Agrodefesa 6/2024`), computes three normalized `[0, 1]` objectives (climate exposure `g1`, vegetation cover discontinuity `g2`, crop diversity deficit `g3`), maps the Pareto frontier (`36` non-dominated plans; `6` distinct crop-signature alternatives), and compares four strategies side by side:
- **Current Plan (`P0`):** User-selected baseline rotation.
- **Conventional Optimization (`P1`):** Best plan under average construction-period climate.
- **Expected-Loss Optimization (`P1b`):** Best average loss across historical construction scenarios.
- **Minimax Regret (`P2`):** Plan minimizing worst-case regret across construction scenarios.

Our contribution is the transparent decision-support tool and its leak-checked evaluation—not a claim that minimax regret is superior.

### How NASA Data Enables This Solution
The application relies on a versioned offline snapshot of the **NASA POWER Daily Point API (`v2.10.3`, `MERRA2` + `POWER`, community `AG`)** for Rio Verde (`-18.0, -50.625`, elevation `625.31 m`), spanning `1981-01-01` to `2025-12-31` (`16,436` daily rows, zero missing `-999` values for `PRECTOTCORR`, `T2M`, `T2M_MAX`, `T2M_MIN`). Multi-decade NASA observations allow us to construct non-overlapping two-year historical climate scenarios, compute drought-exposure and heat/frost indicators, and separate parameter/plan selection (`1981/82–2009/10`, `28` pairs) from held-out retrospective testing (`2010/11–2024/25`, `14` pairs).

### Retrospective Evaluation & Results (Honest Negative Outcome)
- **Verification:** `167` automated tests pass (`pytest -q`, `16.45s`) and `ruff check .` passes cleanly in the development container; independent evaluation (`reports/20261114T181200Z/eval_report.md`) reproduces identical hashes (`eval_result_hash`: `504c28e8...4c18`, `run_result_hash`: `b4370332...fd66`).
- **Overall Out-of-Sample Verdict (`2010/11–2024/25`, weights `0.50 / 0.25 / 0.25`):** **NEGATIVE.** Minimax regret (`P2`) did **not** reduce held-out max regret compared to expected-loss (`P1b`) and did not outperform conventional average-climate optimization (`P1`):
  - **`P2` vs. `P1b` (primary):** **NEGATIVE** — test max regret `0.1744` (`P2`) vs. `0.1466` (`P1b`), median paired difference `D = +0.0064`, 95% CI `[-0.0609, +0.0375]`, Wilcoxon `p = 0.6875` (`7` non-overlapping pairs; `4` wins, `4` ties, `6` losses).
  - **`P2` vs. `P1` (secondary):** **NEGATIVE** — `P2` lowers test max regret (`0.1744` vs. `0.2811`) and has an average-climate price of robustness `M5 = 0.0080`, but performs worse in most test years (`2` wins, `4` ties, `8` losses; median `D = +0.0228`, 95% CI `[+0.0112, +0.0549]`). Using the **mean** CI (`[-0.0234, +0.0466]`) instead of the pre-selected median CI would make this comparison **INCONCLUSIVE**.
  - **`P0` (default baseline):** Viable under ZARC (`<= 20%`), but worst on the held-out test set (max regret `0.3196`, `0` years within `ε = 0.02`).
  - **15-weight sensitivity grid (`P2` vs. `P1b`):** **NEGATIVE** in `8` points and **INCONCLUSIVE** in `7` points (`0` sustained).

### Caution, Limitations & Responsible Use
FieldShift Optimizer **supports—and does not replace—the farmer's judgment** or local agronomic assistance. It makes **no promise of yield, profit, water savings, or soil recovery**, and **no claim of unprecedented scientific novelty**. Key limitations include single-cell climate representation, unverified agronomic proxy thresholds (`docs/LIMITATIONS.md`), exclusion of solar radiation/ET0, low statistical power (`7` bootstrap blocks; `12/14` test pairs below construction `P25`), and unavoidable temporal overlap in external ZARC/PRECTOTCORR calibrations.
```

### Use of SI (Synthetic Intelligence)
```text
The core concept, problem framing, decision-support approach, and non-leaky retrospective evaluation design were conceived by the human team; SI was NOT used to generate the idea of how to solve the challenge.

OpenBot agents (Claude models) assisted with software implementation, test authoring, and English documentation under human specifications and review. SI-assisted source modules are marked `# AI-assisted (OpenBot agent)` in file headers and documented in `docs/AI_USE.md`. No climate or agronomic data values were generated by SI; all values come strictly from the versioned NASA POWER snapshot and cited Brazilian agronomic sources.

Media/Demo SI usage: [RESULT PENDING — state "No SI-generated images, video, or audio were used" OR confirm visible watermarks and absence of NASA branding].
```

### NASA Data
```text
Dataset: NASA POWER (Prediction Of Worldwide Energy Resources) Daily Point API, v2.10.3 (MERRA2 + POWER, community AG)
Coordinates: Latitude -18.0, Longitude -50.625 (Elevation 625.31 m; Rio Verde, GO, Brazil)
Period & Rows: 1981-01-01 to 2025-12-31 (16,436 daily rows; 0 missing -999 values)
Variables: PRECTOTCORR (mm/day), T2M (C), T2M_MAX (C), T2M_MIN (C)
Snapshot ID: power-daily-2.10.3-lat-18_lon-50.625-46c276253b44 (collected 2026-11-14T17:58:06Z)
Required Credit: "The data was obtained from National Aeronautics and Space Administration (NASA) Langley Research Center's Prediction Of Worldwide Energy Resources (POWER) project funded through the NASA Earth Science Division. The data was obtained from the POWER Project's Daily v2.10.3 version on 2026/11/14." (https://power.larc.nasa.gov/docs/referencing/)
```

### Space Agency Partner Data and Other References
```text
Space Agency Partner Data: None used in this MVP prototype.
Other References:
- MAPA/SPA/CGRA ZARC 2026/27 Risk Zoning Table (Rio Verde, IBGE 5218805 slice, CC-BY): https://dados.agricultura.gov.br/dataset/tabua-de-risco-zoneamento-agricola-de-risco-climatico
- MAPA ZARC Ordinances (Portarias SPA/MAPA 368/2025, 178/2025, 386/2025, 400/2025, 123/2025): https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico
- IN Agrodefesa 6/2024 (Goiás Soybean Sanitary Void & Sowing Window): https://goias.gov.br/agrodefesa/wp-content/uploads/sites/49/2014/11/Instrucao-Normativa-Soja_Agrodefesa-No-06-de-05-de-agosto-de-2024-1-1.pdf
- Embrapa Algodão Comunicado Técnico 377 (2016): https://www.infoteca.cnptia.embrapa.br/infoteca/bitstream/doc/1066067/1/Sistemasdecultivodeplantasdecobertura.pdf
- ISRIC SoilGrids 2.0 (context check only): https://rest.isric.org/soilgrids/v2.0/docs
- Software stack: Python 3.12.16, numpy 2.5.4, pandas 3.1.1, pyarrow 26.0.1, streamlit 1.67.0, plotly 7.1.0, pytest 9.2.0, ruff 0.17.5 (full list in docs/REFERENCES.md).
```

---

## 2. Demo Script: 7-Slide Deck OR 30-Second Video

### 2.1 Seven-Slide Deck (Max 7 Slides Including Title)

| Slide | Section | Content |
|---|---|---|
| **1** | **Title & Team** | **FieldShift Optimizer** — Explore and Compare Crop Rotation Trade-offs Under Climate Uncertainty<br>Challenge: *Field Shift: Adapting Farms with NASA Data* (`spaceappschallenge.org/2026/challenges/adapting-farms-with-nasa-data/`)<br>Team: `[RESULT PENDING: member names & roles]`<br>Public App & Repo: `[RESULT PENDING: URLs]` |
| **2** | **The Challenge** | Rainfed 2-year rotation planning in Rio Verde (GO) faces rainfall and dry-spell uncertainty; average-climate plans can struggle in dry years, while worst-case plans can cost performance in typical years. |
| **3** | **How NASA Data Makes It Possible** | NASA POWER Daily API (`v2.10.3`, `1981–2025`, `16,436` days, `PRECTOTCORR`, `T2M_MAX`, `T2M_MIN`) in a versioned offline snapshot enables 28 construction pairs (`1981/82–2009/10`) and 14 held-out test pairs (`2010/11–2024/25`). |
| **4** | **Project Image & Architecture** | `[RESULT PENDING: clean screenshot of the FieldShift Optimizer Streamlit UI / Pareto frontier]`<br>ZARC `<= 20%` filter (`576` feasible plans) + 3-objective scoring (`g1`, `g2`, `g3`) + `P0 / P1 / P1b / P2` comparison + `36` Pareto plans (`6` distinct alternatives). |
| **5** | **User Experience** | 3-step workflow: (1) select soil (`AD1–AD6` & texture) and baseline `P0`; (2) compare strategies & inspect Pareto trade-offs; (3) review 15-weight sensitivity, cost of robustness (`M5 = 0.0080`), assumptions, and data gaps. |
| **6** | **Retrospective Evaluation & Caution** | **Honest Outcome: NEGATIVE.** `P2` vs. `P1b`: max regret `0.1744` vs. `0.1466` (median `D = +0.0064`, CI `[-0.0609, +0.0375]`). `P2` vs. `P1`: `2` wins, `4` ties, `8` losses. `P0` worst (`0.3196`).<br>Caution: supports—does not replace—farmer judgment; no promises of yield, profit, water, or soil; no claim of novelty. |
| **7** | **References, SI Disclosure & Access** | Key sources (NASA POWER, MAPA ZARC, Agrodefesa, Embrapa), SI disclosure summary (`[RESULT PENDING: demo media SI status]`), and public no-login links (`[RESULT PENDING]`). |

### 2.2 Alternative 30-Second Video Script (Max 30s, ~65–75 Words)

| Time | Visual | Narration (English) |
|---|---|---|
| **0:00–0:05** | Title card: `FieldShift Optimizer`, team names `[RESULT PENDING]` | "Rainfed farmers in Rio Verde, Brazil face tough two-year crop rotation choices under uncertain rainfall—where average-year plans and worst-case plans carry real trade-offs." |
| **0:05–0:13** | NASA POWER snapshot card + ZARC calendar diagram | "FieldShift Optimizer combines 45 years of NASA POWER daily climate data with official Brazilian ZARC risk tables in a reproducible offline snapshot." |
| **0:13–0:23** | Live app screen recording: inputs, Pareto chart, policy comparison `[RESULT PENDING]` | "Users compare their current plan against average-climate, expected-loss, and minimax regret strategies across climate exposure, cover continuity, and diversity." |
| **0:23–0:30** | Evaluation card + Caution banner + public URL `[RESULT PENDING]` | "Our held-out test honestly reports a negative result: minimax regret did not beat expected loss. Built to support—never replace—farmer judgment." |

---

## 3. Official Submission Checklist

- [ ] **Team Eligibility:** 2 to 6 human members, all registered in the same **Universal Event**; team owner and timezone confirmed.
- [ ] **Language:** All Project Page fields and demo materials written in **English** (if any video speech is not in English, English subtitles are required).
- [ ] **Demo Limit:** Video is `<= 30 seconds` OR presentation is `<= 7 slides` (including title slide). Exceeding this makes the project ineligible for global judging.
- [ ] **Public Links (No Login):** Both `Link to Demo` and `Link to Final Project` (live Streamlit app + public GitHub repo `Marcux777/fieldshift-optimizer`) tested in a **private/incognito browser window** without login.
- [ ] **FortiGuard Classification:** Submit the newly deployed Streamlit URL through the FortiGuard URL classification form (recommended in Submission Guide).
- [ ] **Mandatory Checkboxes:** `Terms and Conditions` and `Confirm All Work is Original` checked on the submission form.
- [ ] **Media Compliance:** No images, voices, or likeness of minors under 18; no NASA branding/logo in generated content.
- [ ] **SI Disclosure:** Completed in both `Use of SI` text and repository metadata (`docs/AI_USE.md`); visible watermark verified if any SI-generated media is used (`[RESULT PENDING]`).
- [ ] **Final User Approval:** Explicit user sign-off obtained before clicking submit (internal target: 2026-11-15 21:00 BRT; official deadline: 23:59 in team owner's timezone).
