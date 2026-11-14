# NASA Space Apps 2026 — Submission Draft & Checklist

> **Target:** [Field Shift: Adapting Farms with NASA Data](https://www.spaceappschallenge.org/2026/challenges/adapting-farms-with-nasa-data/)
> **Internal Deadline:** 2026-11-15 21:00 BRT (Official Deadline: 2026-11-15 23:59 in Team Owner's timezone).
> **Note:** Do **not** submit externally without explicit user approval. Evaluation metrics and live URLs marked `[RESULT PENDING]` will be filled only from verified `reports/` and live deployment checks.

---

## 1. Project Page Form Fields (English Draft)

### Project Name
```text
FieldShift Optimizer — Robust Crop Rotation Decisions Under Climate Uncertainty
```

### High-Level Summary
```text
FieldShift Optimizer is an open-source decision-support web application for rainfed crop rotation planning in Rio Verde, Goiás, Brazil, over a two-year (four-window) horizon. Using a versioned historical climate snapshot from NASA POWER alongside official Brazilian ZARC sowing-risk tables, it compares a farmer's current rotation against average-climate optimization and minimax regret across climate exposure, cover continuity, and crop diversity. The tool supports—and does not replace—farmer judgment, transparently displaying trade-offs, assumptions, and retrospective out-of-sample results without promising yield, profit, water, or soil outcomes.
```

### Link to Demo
```text
URL: [RESULT PENDING: public URL without login to <= 7-slide deck OR <= 30-second video]
Format: [RESULT PENDING: 7-slide deck / 30-second video]
```

### Link to Final Project
```text
Live App URL: [RESULT PENDING: public Streamlit Community Cloud URL without login]
Public Repository URL: https://github.com/Marcux777/fieldshift-optimizer [RESULT PENDING: verify public access after user creates repo]
```

### Detailed Description
```markdown
### Problem & Central Question
Rainfed farmers in Rio Verde (Goiás, Brazil) choose multi-season crop rotations under substantial rainfall and dry-spell uncertainty. A rotation that looks optimal under average historical weather can suffer severe climate exposure during dry years. FieldShift Optimizer addresses a practical question: *"Which crop rotation represents a more defensible choice when we do not know which climate conditions will occur?"*

### Solution Overview
FieldShift Optimizer is a Streamlit decision-support tool for a two-agricultural-year, four-window (`J1`–`J4`) rainfed planning horizon. Users specify soil parameters (`AD1–AD6` water-availability class and texture), baseline rotation (`P0`), and objective preferences. The engine enumerates all agronomically feasible rotations under official ZARC 2026/27 (`<= 20%` risk) and Goiás legal sowing/sanitary-void rules (`IN Agrodefesa 6/2024`), computes three normalized `[0, 1]` objectives (climate exposure `g1`, vegetation cover discontinuity `g2`, crop diversity deficit `g3`), identifies the Pareto frontier, and compares:
- **Current Plan (`P0`):** User-selected baseline rotation.
- **Conventional Optimization (`P1`):** Best plan under average construction-period climate.
- **Expected-Loss Optimization (`P1b`):** Best average loss across historical construction scenarios.
- **Minimax Regret (`P2`):** Plan minimizing worst-case regret across construction scenarios.

### How NASA Data Enables This Solution
The application relies on a versioned offline snapshot of the **NASA POWER Daily Point API (`v2.10.3`, `MERRA2` + `POWER`, community `AG`)** for Rio Verde (`-18.0, -50.625`, elevation `625.31 m`), spanning `1981-01-01` to `2025-12-31` (`16,436` daily rows, zero missing `-999` values for `PRECTOTCORR`, `T2M`, `T2M_MAX`, `T2M_MIN`). Multi-decade NASA observations allow us to construct non-overlapping two-year historical climate scenarios, compute drought-exposure and heat/frost indicators, and separate parameter/plan selection (`1981/82–2009/10`, 28 pairs) from held-out retrospective testing (`2010/11–2024/25`, 14 pairs).

### Retrospective Evaluation & Results
- **Test Suite Status:** `[RESULT PENDING]`
- **Out-of-Sample Comparison (`2010/11–2024/25`):** `[RESULT PENDING]` (insert verified metrics `M0–M5`, confidence intervals, and honest outcome classification—positive, negative, or inconclusive—from `reports/`).

### Caution, Limitations & Responsible Use
FieldShift Optimizer **supports—and does not replace—the farmer's judgment** or local agronomic assistance. It makes **no promise of yield, profit, water savings, or soil recovery**, and **no claim of unprecedented scientific novelty**. Key limitations include single-cell climate representation, unverified agronomic proxy thresholds (`docs/LIMITATIONS.md`), exclusion of solar radiation/ET0, and unavoidable temporal overlap in external ZARC/PRECTOTCORR calibrations.
```

### Use of SI (Synthetic Intelligence)
```text
The core concept, problem framing, minimax regret decision-support approach, and non-leaky retrospective evaluation design were conceived by the human team; SI was NOT used to generate the idea of how to solve the challenge.

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
| **1** | **Title & Team** | **FieldShift Optimizer** — Robust Crop Rotation Decisions Under Climate Uncertainty<br>Challenge: *Field Shift: Adapting Farms with NASA Data* (`spaceappschallenge.org/2026/challenges/adapting-farms-with-nasa-data/`)<br>Team: `[RESULT PENDING: member names & roles]`<br>Public App & Repo: `[RESULT PENDING: URLs]` |
| **2** | **The Challenge** | Why rainfed 2-year rotation planning in Rio Verde (GO) is vulnerable to rainfall variability and dry spells; why an average-climate optimum can underperform in dry years. |
| **3** | **How NASA Data Makes It Possible** | NASA POWER Daily API (`v2.10.3`, `1981–2025`, `16,436` days, `PRECTOTCORR`, `T2M_MAX`, `T2M_MIN`) in a versioned offline snapshot enables multi-decade historical scenarios and strict out-of-sample testing (`1981–2010` vs. `2010–2025`). |
| **4** | **Project Image & Architecture** | `[RESULT PENDING: clean screenshot of the FieldShift Optimizer Streamlit UI / Pareto frontier]`<br>ZARC `<= 20%` feasibility filter + 3-objective evaluation (`g1` exposure, `g2` cover, `g3` diversity) + `P0 / P1 / P1b / P2` comparison. |
| **5** | **User Experience** | 3-step workflow: (1) select soil (`AD1–AD6` & texture) and current baseline `P0`; (2) compare strategies & inspect Pareto alternatives; (3) review sensitivity across 15 weightings, assumptions, and data gaps. |
| **6** | **Retrospective Evaluation & Caution** | `[RESULT PENDING: verified test-period findings from reports/]`<br>Explicit caution: supports—does not replace—farmer judgment; no promises of yield, profit, water, or soil; no claim of novelty. |
| **7** | **References, SI Disclosure & Access** | Key sources (NASA POWER, MAPA ZARC, Agrodefesa, Embrapa), SI disclosure summary, and public no-login links (`[RESULT PENDING]`). |

### 2.2 Alternative 30-Second Video Script (Max 30s, ~65–75 Words)

| Time | Visual | Narration (English) |
|---|---|---|
| **0:00–0:05** | Title card: `FieldShift Optimizer`, team names `[RESULT PENDING]` | "Rainfed farmers in Rio Verde, Brazil face hard two-year crop rotation choices under uncertain rainfall—where average-year plans can struggle in dry years." |
| **0:05–0:13** | NASA POWER snapshot card + ZARC calendar diagram | "FieldShift Optimizer combines 45 years of NASA POWER daily climate data with official Brazilian ZARC risk tables in a reproducible offline snapshot." |
| **0:13–0:23** | Live app screen recording: inputs, Pareto chart, policy comparison `[RESULT PENDING]` | "Farmers compare their current plan against average-climate optimization and minimax regret across climate exposure, cover continuity, and crop diversity." |
| **0:23–0:30** | Evaluation card + Caution banner + public URL `[RESULT PENDING]` | "`[RESULT PENDING: 1-sentence verified evaluation takeaway]`. Built to support—never replace—farmer judgment. Try it at our public link." |

---

## 3. Official Submission Checklist

- [ ] **Team Eligibility:** 2 to 6 human members, all registered in the same **Universal Event**; team owner and timezone confirmed.
- [ ] **Language:** All Project Page fields and demo materials written in **English** (if any video speech is not in English, English subtitles are required).
- [ ] **Demo Limit:** Video is `<= 30 seconds` OR presentation is `<= 7 slides` (including title slide). Exceeding this makes the project ineligible for global judging.
- [ ] **Public Links (No Login):** Both `Link to Demo` and `Link to Final Project` (live Streamlit app + public GitHub repo `Marcux777/fieldshift-optimizer`) tested in a **private/incognito browser window** without login.
- [ ] **FortiGuard Classification:** Submit the newly deployed Streamlit URL through the FortiGuard URL classification form (recommended in Submission Guide).
- [ ] **Mandatory Checkboxes:** `Terms and Conditions` and `Confirm All Work is Original` checked on the submission form.
- [ ] **Media Compliance:** No images, voices, or likeness of minors under 18; no NASA branding/logo in generated content.
- [ ] **SI Disclosure:** Completed in both `Use of SI` text and repository metadata (`docs/AI_USE.md`); visible watermark verified if any SI-generated media is used.
- [ ] **Final User Approval:** Explicit user sign-off obtained before clicking submit (internal target: 2026-11-15 21:00 BRT; official deadline: 23:59 in team owner's timezone).
