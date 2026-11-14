# References & Data Provenance

All datasets, agronomic documents, software libraries, and official challenge resources used in FieldShift Optimizer are listed below. Each entry includes a verifiable URL and its current verification status (`[V]` verified at source; `[N]` unverified / hypothesis).

---

## 1. NASA Data

### `[REF-01]` NASA POWER Daily Point API (`v2.10.3`)
- **Provider:** NASA Langley Research Center (LaRC) — Prediction Of Worldwide Energy Resources (POWER), funded through the NASA Earth Science Division.
- **Sources:** `MERRA2`, `POWER` (Community: `AG`, Time Standard: `LST`).
- **Spatial Point Requested & Returned:** Latitude `-18.0`, Longitude `-50.625`, Elevation `625.31 m` (grid cell representing Rio Verde, GO, Brazil).
- **Temporal Coverage:** `1981-01-01` to `2025-12-31` (`16,436` daily rows; `0` missing `-999.0` fill values).
- **Parameters & Units:** `PRECTOTCORR` (`mm/day`), `T2M` (`C`), `T2M_MAX` (`C`), `T2M_MIN` (`C`). (`T2M` is stored in the snapshot but not used in the loss indicators.)
- **Collection Timestamp (UTC):** `2026-11-14T17:58:06Z`
- **Snapshot ID:** `power-daily-2.10.3-lat-18_lon-50.625-46c276253b44` (`data/snapshots/power-daily-2.10.3-lat-18_lon-50.625-46c276253b44/manifest.json`)
- **Snapshot File Hashes (SHA-256):**
  - `raw/power_daily.json.gz`: `2c5539841f759d06da232a174faaa300fb9370541facb103db62135bd62f6e32` (content hash without volatile `times` field: `46c276253b4435ba8e996b208fc953365b5ae13d845296b4f5dc37f0369095fe`)
  - `daily.parquet`: `0025a0aedcfed547fe5d02dfefebfbbd5659b95e34ef770d1bfcb1d8aa689c7f`
- **Request URL:** `https://power.larc.nasa.gov/api/temporal/daily/point?parameters=PRECTOTCORR,T2M,T2M_MAX,T2M_MIN&community=AG&longitude=-50.625&latitude=-18&start=19810101&end=20251231&format=JSON`
- **Referencing Guide URL:** https://power.larc.nasa.gov/docs/referencing/
- **Required Credit Text (from `manifest.json`):**
  > *"The data was obtained from National Aeronautics and Space Administration (NASA) Langley Research Center's Prediction Of Worldwide Energy Resources (POWER) project funded through the NASA Earth Science Division."*
  > *"The data was obtained from the POWER Project's Daily v2.10.3 version on 2026/11/14."*
- **License Note:** No formal open-source license text was located on the NASA POWER referencing page; attribution follows the official referencing guide above.
- **Status:** `[V]`

---

## 2. Space Agency Partner Data

- **Status:** None used in this MVP prototype. (Brazilian Space Agency / INPE and U.S.-centric datasets such as SSURGO, CDL, and PRISM are not used; NASA IMERG, SMAP, and MODIS were excluded because they require Earthdata authentication.)

---

## 3. Agronomic & Soil Sources (Brazil / Rio Verde, GO)

### `[REF-02]` ZARC Risk Zoning Table — 2026/27 Harvest (Rio Verde, GO Slice)
- **Provider:** Ministério da Agricultura e Pecuária (MAPA / SPA / CGRA) — Dados Abertos.
- **License:** CC-BY.
- **Dataset Page:** https://dados.agricultura.gov.br/dataset/tabua-de-risco-zoneamento-agricola-de-risco-climatico
- **National CSV URL (accessed 2026-11-13):** https://dados.agricultura.gov.br/dataset/6d3d141c-885e-41a4-ab7f-dc8ff323b96f/resource/139e5a60-1f43-4cc8-aeab-a35dbbf816c0/download/dados-abertos-tabua-de-risco-safra-2026-2027.csv
  - National CSV SHA-256: `b5d242f78571d4694c843f8a5f8fcad4dc10c1c83192ebae2d40b78cc5f5efe3`
- **Local Filtered Slice (`geocodigo = 5218805`, 251 data rows):** `config/zarc/zarc-tabua-rio-verde-5218805-2026-2027.csv`
  - Slice SHA-256: `88f24986b6063e0eaba7bd0cd83f97d9a623f5c5f9ada156711e318b9e3218ad`
- **Data Dictionary PDF:** https://dados.agricultura.gov.br/dataset/6d3d141c-885e-41a4-ab7f-dc8ff323b96f/resource/bebb0ebb-bc75-460c-b900-a7a1ebd87bee/download/dicionario-de-dados-tabua-de-risco-2026.pdf
  - PDF SHA-256: `2143127cd635744e54b496a876e80be3e53bc0d1508b8885e321b2bc35d4917c`
- **Used For:** 20% risk sowing decêndios by crop, cycle group (`Cod_Ciclo = 20`), rainfed management (`Cod_Outros_Manejos = 1`), and soil code.
- **Status:** `[V]`

### `[REF-03]` MAPA ZARC Ordinances (Portarias SPA/MAPA — Goiás)
- **Provider:** Ministério da Agricultura e Pecuária (MAPA).
- **Portal URL:** https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico
- **Ordinances Used:**
  - **Soybean (GO):** Portaria SPA/MAPA 368/2025 (parameters: cycles 100/115/130 d, AD1–AD6, `Tmax > 40 °C`, `Tmin <= 3 °C`, emergence 5–10 d) — https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico/portarias/2025-2026/goias-go/PORTN368SOJAGO.ret2.pdf (2026/27 table via Portaria 198/2026, 2026-07-03) `[V]`
  - **1st-Season Corn (GO):** Portaria SPA/MAPA 178/2025 (Group I: 100 d cycle, phases 15/35/30/20 d, frost `Tmin <= 2 °C`) — https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico/portarias/2025-2026/goias-go/PORTN178MILHO1SAFRAGO.ret2.pdf (2026/27 table via Portaria 221/2026, 2026-07-10) `[V]`
  - **2nd-Season Corn (GO):** Portaria SPA/MAPA 386/2025 (Group I `<= 110 d`, frost `<= 2 °C`, `>= 6` rainy days in final decêndio) — https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico/portarias/2025-2026/goias-go/copy_of_PORTN33.PDF (2026/27 table via Portaria 322/2026, 2026-07-30) `[V]`
  - **2nd-Season Corn + Brachiaria Intercrop (GO):** Portaria SPA/MAPA 400/2025 (Group I `<= 115 d`) — https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico/portarias/2025-2026/goias-go/PO91CB1.PDF (2026/27 table via Portaria 337/2026, 2026-08-06) `[V]`
  - **2nd-Season Grain Sorghum (GO):** Portaria SPA/MAPA 123/2025 (Group I `<= 110 d`, `Tmax > 38 °C`, `Tmin < 2 °C`) — https://www.gov.br/agricultura/pt-br/assuntos/riscos-seguro/programa-nacional-de-zoneamento-agricola-de-risco-climatico/portarias/2025-2026/goias-go/PORTN123SORGOGRANIFEROGO.pdf (2026/27 table via Portaria 273/2026, 2026-07-22) `[V]`

### `[REF-04]` IN Agrodefesa 6/2024 (Goiás Soybean Sanitary Void & Sowing Calendar)
- **Provider:** Agência Goiana de Defesa Agropecuária (Agrodefesa).
- **URL:** https://goias.gov.br/agrodefesa/wp-content/uploads/sites/49/2014/11/Instrucao-Normativa-Soja_Agrodefesa-No-06-de-05-de-agosto-de-2024-1-1.pdf
- **Used For:** Legal soybean sowing window in Goiás (`Sep 25` to `Jan 02`) and sanitary void (`Jun 27` to `Sep 24`).
- **Status:** `[V]` (official text read 2026-11-14). Note: `IN Agrodefesa 5/2026` (exceptional Sep 20 start for 2026/27) was found only in news reports (https://comprerural.com/agrodefesa-antecipa-inicio-da-semeadura-da-soja-em-goias-para-20-de-setembro/) `[N]` and has no effect on the model because `dec27` has 40% ZARC risk.

### `[REF-05]` Embrapa Algodão — Comunicado Técnico 377 (2016)
- **Title:** *Sistemas de cultivo de plantas de cobertura* (context on sorghum drought tolerance/earliness and brachiaria cover).
- **URL:** https://www.infoteca.cnptia.embrapa.br/infoteca/bitstream/doc/1066067/1/Sistemasdecultivodeplantasdecobertura.pdf
- **Used For:** Agronomic context for 2nd-season sorghum and corn+brachiaria intercropping.
- **Status:** `[V]`

### `[REF-06]` Embrapa Soja — Biological Nitrogen Fixation (Context Only)
- **URL:** https://www.infoteca.cnptia.embrapa.br/infoteca/bitstream/doc/447053/1/CNPSOCOM.TEC.7400.pdf
- **Used For:** Background agronomic context only (not parameterized in the numerical engine).
- **Status:** `[N]` (not read in full)

### `[REF-07]` ISRIC SoilGrids 2.0 REST API (Context Check Only)
- **URL:** https://rest.isric.org/soilgrids/v2.0/docs
- **Used For:** Context check on 2026-11-13 showing 29–51% clay at two rural points in Rio Verde (not used in the optimization engine; soil class is user-supplied with default hypothesis `AD4 + clayey`).
- **Status:** `[V]`

---

## 4. Software Libraries (Verified from `uv.lock`)

| Package | Version (`uv.lock`) | Purpose | URL |
|---|---|---|---|
| Python | `3.12.16` | Runtime (`requires-python >= 3.12`) | https://www.python.org/ |
| `numpy` | `2.5.4` | Vectorized indicators, loss/regret matrices | https://numpy.org/ |
| `pandas` | `3.1.1` | Climate time-series & ZARC tabular handling | https://pandas.pydata.org/ |
| `pyarrow` | `26.0.1` | Parquet snapshot storage | https://arrow.apache.org/docs/python/ |
| `streamlit` | `1.67.0` | Web application interface | https://streamlit.io/ |
| `plotly` | `7.1.0` | Interactive Pareto & comparison charts | https://plotly.com/python/ |
| `pytest` | `9.2.0` | Unit, leakage, and AppTest suite (dev) | https://docs.pytest.org/ |
| `ruff` | `0.17.5` | Linting and formatting (dev) | https://docs.astral.sh/ruff/ |

---

## 5. Official NASA Space Apps 2026 Resources

- **Challenge Page (*Field Shift: Adapting Farms with NASA Data*):** https://www.spaceappschallenge.org/2026/challenges/adapting-farms-with-nasa-data/ `[V]`
- **Project Submission Guide:** https://www.spaceappschallenge.org/resources/project-submission-guide/ `[V]`
- **Judging & Awards Guide:** https://www.spaceappschallenge.org/resources/judging-awards-guide/ `[V]`

---

## 6. Demo & Visual Assets

- Application screenshots captured directly from the FieldShift Optimizer Streamlit interface (`docs/assets/`): `app-pareto.png`, `app-negative-evaluation.png`, `app-nasa-sources.png`, `app-ad1-p0-infeasible.png`.
- `[RESULT PENDING]` (Final 7-slide deck or 30-second video media link and any additional external visual/audio assets, if used.)
