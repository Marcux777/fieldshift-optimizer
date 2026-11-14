# Contrato de execução da migração Rust + SvelteKit (14/11/2026)

Autoridade, em ordem: pedido do usuário (execução integral E0–E9) > `OpenBot/Shared/nasa-spaceapps-2026-fieldshift/rust-migration/PLANO-MIGRACAO-RUST.md` > este contrato > `docs/RUST_MIGRATION.md`.
Baseline científico: commit `16cc391318b57fbf44a31c75c994fbb0683b3115` (Python). Protocolo v1.1 (`docs/EVALUATION_PROTOCOL.md`, SHA-256 `ee8e51c5…aefb9`), desfecho NEGATIVO, EvalReport `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18`, RunResult `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66`.

## Regras inegociáveis
1. Branch `feat/rust-svelte-migration`, worktree `/home/marcusvinicus/p/fieldshift-optimizer-rust`. **`main` e o checkout `/home/marcusvinicus/p/fieldshift-optimizer` não são tocados.** Sem merge, sem deploy, sem submissão, sem push de `main`.
2. **Nada de Python no backend final**: nem processo auxiliar, nem FFI, nem fallback. O Python existe só como oráculo de teste (`tools/reference/`, contêiner `fieldshift-oracle`) e como baseline em `src/`; a remoção é a etapa E11, que exige autorização.
3. **Resultado histórico nunca substitui cálculo.** Os arquivos de `reports/` e as fixtures são oráculos de teste. Os endpoints e o CLI calculam; o que não estiver portado devolve `501`/erro explícito. É proibido ler `reports/` ou as fixtures para produzir uma resposta do produto.
4. **Artefatos congelados não mudam**: `docs/EVALUATION_PROTOCOL.md`, `reports/**`, `config/**`, `data/snapshots/**`. As saídas do Rust vão para `artifacts/rust/<run_id>/` (novo), nunca por cima das do Python.
5. **Divergência** = investigar a causa antes de qualquer tolerância. Classificar como:
   - *serialização*: os valores numéricos são iguais depois de ler o JSON, só o texto ou o hash mudam;
   - *científica*: qualquer diferença em IDs de plano, escolhas, ordem, contagens, desfecho, ou em valores acima de 1e-12.
   Regra de aceite: IDs, ordem, contagens, escolhas e desfecho **idênticos**; floats |Δ| ≤ 1e-12 salvo causa investigada e documentada; hashes **idênticos** como meta. Divergência de serialização que não puder ser eliminada é documentada com a prova de que os valores são iguais; divergência científica **bloqueia** a etapa.
6. Todo código novo leva o cabeçalho `// AI-assisted (OpenBot agent)`.
7. Sem fallback silencioso, sem dado sintético apresentado como real, sem `unwrap()` em caminho de produção; `unsafe` proibido (lint do workspace).

## Ambiente
- Rust: cargo do host (1.98). Cada dono usa seu `CARGO_TARGET_DIR` próprio para não travar o build dos outros: `export CARGO_TARGET_DIR=/tmp/fs-target-<papel>`. Gates locais: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` (em `backend/`).
- Front: Node 22 e npm do host, em `frontend/`.
- Oráculo Python (só fixtures, **sem rede**): `cd /home/marcusvinicus/p/fieldshift-optimizer-rust && export DOCKER_SHARED_CACHE_ROOT=$HOME/.cache/docker-shared && docker compose -f compose.oracle.yaml exec -T oracle <cmd>`. Reutilize o contêiner `fieldshift-oracle`; não crie outro. Verificado: `170 passed` nele. Não use o `fieldshift-dev` (ele monta o checkout de `main`).
- **Commits: só o Development Lead.** Dono: avise no canal "pronto para commit" com os paths e a saída real dos testes; não faça `git add/commit/checkout/stash`. Edite só os seus arquivos.

## Layout e donos
| Caminho | Dono | Etapa |
|---|---|---|
| `tools/reference/**`, `fixtures/reference/**` (geradores e fixtures de modelo/avaliação) | Evaluation & Benchmark Engineer | E0 |
| `backend/crates/fieldshift-numeric/**` (`num`, `rng`, `canon`, `calendar`), `fixtures/reference/numeric_*.json` | MLOps & Data Engineer | E1 |
| `backend/crates/fieldshift-io/**`, `backend/crates/fieldshift-cli/**` | MLOps & Data Engineer | E2 |
| `backend/crates/fieldshift-model/**` (`catalog`, `indicators`, `scenarios`, `optimization`, `selection`, `pareto`) | Optimization Engineer | E3–E5 |
| `backend/crates/fieldshift-eval/**` (`evaluation`, `stats`), `scripts/` de comparação | Evaluation & Benchmark Engineer | E6 |
| `backend/crates/fieldshift-api/**`, `frontend/**` | Software Engineer | E7–E8 |
| `.github/workflows/**`, `backend/Dockerfile`, `compose*.yaml`, Cargo.toml do workspace | MLOps & Data Engineer (CI/Docker), Dev Lead (workspace) | E9 |
| `README.md` da raiz, `docs/RUN.md`, `docs/MIGRATION_REPORT.md` | Scientific Writer, ao final | — |
Crates: `numeric` ← `model` ← `eval`; `io` ← `cli`/`api`. Quem precisar de função de outro dono pede pelo canal; antes de a função existir, programe contra a assinatura abaixo.

## Assinaturas de `fieldshift-numeric` (o MLOps publica as finais aqui, na seção "numeric")
- `num::fsum(&[f64]) -> f64` = `math.fsum` (soma exata, Shewchuk). **Não** usar soma de Neumaier.
- `num::py_round(x: f64, nd: i32) -> f64` = `round(x, nd)` do Python (arredondamento decimal exato, empate ao par).
- `num::np_round(x: f64, nd: i32) -> f64` = `np.round(x, nd)` = `rint(x·10^nd)/10^nd`. **São duas funções diferentes e cada ponto de chamada usa a do Python original** (`optimization.py:384,418,440` usam `round`; `q()` de `optimization.py:244` e `evaluation.py:323` usam `np.round`).
- `num::quantile_linear(&[f64], q: f64) -> f64` = `np.quantile` (método linear); `num::median`, `num::nanmedian`, `num::nanmean`.
- `rng::Pcg64::from_seed(seed: u64)`, `integers(low, high, n) -> Vec<i64>` = `np.random.default_rng(seed).integers(low, high, size)` em ordem de linha, **bit a bit**.
- `canon::to_canonical(&serde_json::Value, opts) -> String` com as opções de `json.dumps` usadas (`sort_keys`, `separators`, `ensure_ascii`, `allow_nan=False`) e floats no estilo `repr` do Python (`1e-05`, `1e+16`, `0.1`); `canon::sha256_hex(&[u8])`.
- `calendar`: data civil, ordinal, ano agrícola set–ago, descarte de 29/fev.

## Fixtures de referência (E0)
Geradas **só** pelo Python do commit `16cc391` no contêiner `fieldshift-oracle`, em `fixtures/reference/`, com `MANIFEST.json` (SHA-256 de cada arquivo, commit, versões numpy/pandas, comando). Formato JSON com floats em `repr` (ida e volta exata). Conteúdo mínimo:
`numeric_vectors` (fsum, rounds, quantile, mediana, ≥1e6 sorteios do PCG64 por hash e amostras), `canon_vectors`, `catalog_AD1..AD6` (IDs, specs, rejeições), `p0_status`, `scenarios` (split, pares válidos, hash do clima médio, medianas de e3), `components_default` (g1,g2,g3 por plano×cenário), `selections_15w` (P1, P1b, P2 e objetivos por vetor de pesos), `pareto` (36 vetores, 50 planos, equivalentes, alternativas), `eval_intermediates` (matrizes L e R do teste, índices do bootstrap, ICs, Wilcoxon, M0–M5), e os goldens `run_result` e `eval_report` (cópias de `reports/20261114T184118Z`).
Testes diferenciais ficam em `backend/crates/*/tests/differential_*.rs` e leem as fixtures; os testes de unidade portam os 170 testes Python por módulo.

## Gates
Um módulo só é "pronto" com: (a) testes portados do Python, passando; (b) teste diferencial contra a fixture, passando; (c) mutações relevantes falham (M-1 e M-2 do avaliador mantidos); (d) `fmt`, `clippy -D warnings`, `cargo test` verdes; (e) relato no canal com a saída real. Nenhum teste isolado nem build verde prova paridade: a paridade exige E3–E6 completos.
Pronto para revisão independente (VSDD Reviewer): E0–E9 completos, CI verde, front testado em desktop e celular, relatório em `docs/MIGRATION_REPORT.md` com divergências remanescentes.

## Interfaces por módulo (cada dono publica as assinaturas finais abaixo, na sua seção, assim que existirem)
### numeric
Publicado: `num::{fsum, py_round(f64,i32), np_round(f64,i32), median, nanmedian, nanmean, quantile_linear, np_sum, np_mean}`. `canon::{sha256_hex, py_float_repr, to_canonical(&Value) /*ASCII*/, to_canonical_utf8(&Value)}`. `rng::Pcg64::{from_seed, integers_below(high:u32,n), integers(low:i64,high:i64,n)->Result<Vec<i64>, &str>}`. `calendar::Date::{new,parse,ordinal,agricultural_year,agricultural_day,iso}`. Diferencial numeric: 4 testes, 1.030.009 sorteios + 70.000 bootstrap, 16.436 datas, rounds/quantis/canon. APIs de rounds/quantis esperam domínio numérico finito do motor; median/nanmedian/nanmean tratam NaN explicitamente.
### io
Publicado: `snapshot::{latest_snapshot(&Path), read_manifest, verify_manifest, load_climate(&Path)->Result<ClimateData,SnapshotError>, parse_power_response, content_sha256, write_snapshot}`. `ClimateData { dates:Vec<calendar::Date>, values:Vec<[f64;4]> }`, ordem PRECTOTCORR/T2M/T2M_MAX/T2M_MIN; ausência=NaN; `.quality_report()` e `.validate()`. Leitura bruta gzip, sem rede, hashes/SHA256SUMS e unidades conferidos; schema 1 congelado inclui hash de Parquet sem interpretá-lo; schema 2 novo é raw-only e nunca sobrescreve baseline. `config::{load_toml,load_zarc}`. Testes de dados: 10 grupos com casos parametrizados adversos e todas as 65.744 células contra o oráculo.
### model
_(Optimization Engineer, `backend/crates/fieldshift-model`, publicado em 14/11 — pronto para uso por eval/api/cli)_

Porte fiel de `catalog.py`, `indicators.py` e da parte de cenários/otimização de `optimization.py` (commit `16cc391`). Núcleo puro: recebe **bytes/texto** de configuração e um `ClimateFrame`; não lê disco nem rede. Erros tipados (`CatalogError`, `IndicatorError`, `ScenarioError`, `OptimizationError`; todos `String` no campo 0, mensagens iguais às do Python).

- `catalog`: `load_catalog(&CatalogSources{catalog_toml, soil_toml, zarc_csv}) -> Catalog` (confere o SHA-256 da tábua), `load_soil(soil_toml, ad_class, texture) -> Soil`, `enumerate_plans(&Catalog, &Soil, 0.20) -> (Vec<Plan>, Vec<Rejection>)` (ordenado por `Plan.id: [i32; 6]`; recusa risco > 0,20), `verify_plan`, `resolve_spec`, `describe_plan -> serde_json::Value`, `fallow_summary`, `diagnose_infeasible`, `find_plan`/`plan_index`, calendário (`md_to_t`, `t_to_dec`, `dec_start_t`, ...). `Plan.occupations: [Occupation; 4]` com `sow_t/emerge_t/harvest_t: Option<i32>` (dia-índice, 0 = 1/set; 365 dias/ano; 730 no horizonte).
- `scenarios`: `ClimateFrame` (datas únicas; 4 variáveis na ordem `VARIABLES`; ausente = NaN), `build_scenarios(&ClimateFrame, Split::{Construction, Test}) -> ScenarioSet` (fatia por split ANTES de qualquer cálculo; 29/fev descartado; cenário com qualquer NaN ou dia ausente é inválido), `construction_climatology(&ScenarioSet) -> (ConstructionClimatology, ClimatePair /*s_barra*/)`. `ClimatePair` guarda 730 dias de P, TX, TN.
- `indicators`: `IndicatorParams::from_toml(bytes, &Catalog)`, `.with_reference(ConstructionClimatology)` (mediana de e3 só da construção), `loss_components(&Plan, &ClimatePair, &IndicatorParams) -> [g1, g2, g3]` (pura), `occurrence_indicators` (e1..e5), `np_sum` (soma em pares do numpy; necessária para `sum(P[a:b]) < 20` bit a bit).
- `selection`: `Tensor3` (plano × cenário × 3), `Matrix`, `weight_grid()` (15 pesos), `loss_matrix`, `regret_matrix`, `select_mmr/conv/exp -> Selection{index, tied_after_level, id_tiebreak}`, `q = np.round(v, 9)`.
- `pareto`: `objective_vectors`, `pareto_front`, `select_alternatives`.
- `optimization`: `run(&ClimateFrame, &Catalog, &Soil, &IndicatorParams, snapshot_id, &RunOptions) -> RunResult` (`status()`: `OK | INVIAVEL | DADOS_INSUFICIENTES`; `body`/`to_value(include_meta)` = o mesmo dicionário do Python; `result_hash()` = SHA-256 do JSON canônico sem `meta`; `policy_plan_id("P2")`), `components_tensor`, `compute_config_hash`.
- `compat`: re-exports diretos de `fieldshift_numeric::num::{fsum, median, np_round, np_sum, py_round}` e `fieldshift_numeric::canon::{py_float_repr, sha256_hex, to_canonical}` (sem duplicação de implementação).
- **Importante para todo o workspace:** `serde_json` precisa da feature `float_roundtrip` para ler floats do oráculo sem erro de 1 ulp (o modelo já a habilita; a unificação de features vale para o workspace inteiro).
- Testes diferenciais (`tests/differential_*.rs`, bit a bit, sem tolerância): 8 solos × (IDs, ordem, datas, rejeições, P0), cenários/split/s_barra/medianas de e3, 74 mil componentes g1–g3 (construção, teste e s_barra), 15 vetores de pesos × P1/P1b/P2 com chaves de desempate, F/Pareto/equivalentes/alternativas, e o `RunResult` inteiro contra `golden_run_result.json` com o hash `b4370332…fd66`.
### eval
Publicado (`backend/crates/fieldshift-eval`):
- `evaluation`: `evaluate_plans(&EvalInput, &mut dyn LossModel, &EvalProtocol) -> Result<EvalReport, EvalError>` (avaliador independente puro; recalcula perdas a partir do plano e do clima sem receber score do otimizador), `split_scenarios(&ClimateDaily) -> Result<ScenarioSplit, EvalError>`, `EvalInput`, `EvalProtocol` (`default_v1_1()`), `EvalReport::{to_value(), result_hash(), to_markdown()}`, `weight_grid()`, `PolicyChoices`.
- `adapter`: `IndicatorLossModel::new(&[Plan], &IndicatorParams)` (ajusta referências de `e3` exclusivamente nos pares de construção e invalida cache por identidade/conteúdo do cenário — M-1), `plan_infos(&Catalog, &[Plan])`, `plan_checker(&Catalog, &Soil)`.
- `stats`: `block_bootstrap_ci(&[f64], &BootstrapConfig) -> BootstrapCi` (blocos de 2 pares, 10.000 réplicas, `seed=20261114`, `fieldshift_numeric::rng::Pcg64` bit a bit com NumPy), `wilcoxon_exact(&[f64]) -> WilcoxonResult` (enumeração exata 2ⁿ), `classify(...) -> Outcome` (`NEGATIVO | INCONCLUSIVO | SUSTENTADO | INVIAVEL | DADOS_INSUFICIENTES`).
- Testes (30 no crate): 6 unitários (`stats`, `npcompat`), 14 diferenciais/regressão (`tests/differential_eval.rs`, incluindo M-1, M-2, Markdown e `EvalReport` completo com hash `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18`), 1 diferencial RNG (`tests/differential_rng.rs`, 70.000 índices bit a bit) e 9 testes de protocolo/gates (`tests/port_evaluation.rs`).
### api
Publicado (`backend/crates/fieldshift-api` + `frontend/`):
- `engine::Engine`: `load(&Path) -> Result<Self>` (carrega configs + snapshot mais recente validado via `fieldshift_io` + protocolo v1.1 sem rede e sem ler `reports/` ou `fixtures/`), `plans(&RunInput) -> Result<Value, ApiError>`, `run(&RunInput) -> Result<Value, ApiError>`, `evaluate(&RunInput) -> Result<Value, ApiError>`, `pareto(&RunInput) -> Result<Value, ApiError>`, `meta()`, `catalog_meta()`, `provenance()`, `limitations()`.
- `router(engine, static_dir, cors_origins) -> Router`: rotas `GET /health`, `GET /v1/health`, `GET /v1/meta`, `GET /v1/catalog`, `GET /v1/provenance`, `GET /v1/limitations`, `GET /v1/openapi.json`, `POST /v1/plans`, `POST /v1/run`, `POST /v1/evaluate`, `POST /v1/pareto`, mais fallback `ServeDir` + `200.html` quando `FIELDSHIFT_STATIC_DIR` está definido; limite de payload `65536` bytes (`413 PAYLOAD_TOO_LARGE`), `415 UNSUPPORTED_MEDIA_TYPE`, `400 MALFORMED_JSON`, `422 INVALID_INPUT`, `404 NOT_FOUND`, `405 METHOD_NOT_ALLOWED`.
- Contrato e cliente tipado: `backend/crates/fieldshift-api/openapi.json` → gerador `frontend/scripts/generate-openapi-client.mjs` → `frontend/src/lib/api/openapi.gen.ts` + cliente de fronteira `frontend/src/lib/api/client.ts` (tipos nominais `Sha256Hex`, `SnapshotId` e validação de invariantes antes da requisição e na resposta).
- Testes E7/E8: 5 suítes de integração/diferenciais em `backend/crates/fieldshift-api/tests/api_integration.rs` (paridade bit a bit dos 8 catálogos, `RunResult` `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66`, `EvalReport` `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18`, diagnóstico exploratório `AD1` e todos os casos de `test_pipeline.py`), 5 testes unitários Vitest em `frontend/src/lib/api/client.test.ts` e 6 testes E2E Playwright (`desktop-chromium` 1280×800 e `mobile-chromium` 390×844) em `frontend/e2e/app.spec.ts` contra o binário Rust real.
