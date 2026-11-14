# Contrato de desenvolvimento (14/11/2026)

Autoridade: `docs/DECISIONS.md` > `docs/EVALUATION_PROTOCOL.md` (v1.1 congelada) > seções "Revisão 13/11 noite" de `docs/planning/prep/*.md` > restante das notas.

## Ambiente (um contêiner reutilizável; nunca crie outro)
- Subir/reutilizar: `cd /home/marcusvinicus/p/fieldshift-optimizer && export DOCKER_SHARED_CACHE_ROOT=$HOME/.cache/docker-shared && docker compose up -d dev`
- Executar: `docker compose exec -T dev <cmd>` (ex.: `pytest -q`, `ruff check .`, `python -m fieldshift...`). O checkout está montado em `/app`; o venv está em `/opt/venv`.
- Dependência nova: `docker compose exec -T dev uv add <pkg>` e avisar o Development Lead. Só numpy, pandas, pyarrow, streamlit, plotly, pytest, ruff por padrão.
- Rede: só o coletor do snapshot usa rede. Núcleo, testes e app rodam offline.

## Regras de trabalho
- Cada membro edita SÓ os arquivos que possui (abaixo). Não faça `git add/commit/checkout`: o Development Lead faz os commits. Precisa de mudança em arquivo alheio: peça ao dono pelo canal.
- Núcleo (`catalog`, `indicators`, `optimization`, `evaluation`) sem I/O, rede ou Streamlit; adaptadores fazem I/O. Sem fallback silencioso nem dado sintético apresentado como real.
- Cada entrega traz testes que passam no contêiner, com a saída real colada no relato. Sem evidência, não está pronto.
- Resultado negativo ou inconclusivo se reporta como está. Sem promessa de produtividade, lucro, água ou solo.
- Código gerado com SI: marque no cabeçalho do módulo `# AI-assisted (OpenBot agent)`.

## Dono de cada arquivo
| Dono | Arquivos |
| --- | --- |
| MLOps & Data Engineer | `src/fieldshift/data.py`, `scripts/collect_snapshot.py`, `data/snapshots/**`, `tests/test_data.py` |
| Methods Scientist | `config/catalog.toml`, `config/indicators.toml`, `config/soil.toml`, `docs/DATA_SOURCES.md`, `docs/AGRONOMY.md` |
| Optimization Engineer | `src/fieldshift/catalog.py`, `indicators.py`, `optimization.py`, `tests/test_catalog.py`, `test_indicators.py`, `test_optimization.py` |
| Evaluation & Benchmark Engineer | `src/fieldshift/evaluation.py`, `scripts/run_evaluation.py`, `reports/**`, `tests/test_evaluation.py`, `tests/test_leakage.py` |
| Software Engineer | `app.py`, `src/fieldshift/pipeline.py`, `src/fieldshift/ui_*.py`, `.streamlit/config.toml`, `tests/test_app.py`, `tests/test_pipeline.py` |
| Scientific Writer | `README.md`, `docs/SUBMISSION.md`, `docs/REFERENCES.md`, `docs/LIMITATIONS.md`, `docs/AI_USE.md` |
| Development Lead | `pyproject.toml`, `uv.lock`, `Dockerfile`, `compose.yaml`, `AGENTS.md`, `docs/DECISIONS.md`, `docs/CONTRACT.md`, `docs/PROGRESS.md`, commits |

## Interfaces (assinaturas; o dono pode refinar e DEVE publicar o que mudar neste arquivo, na seção do seu módulo)
- `data.load_climate(snapshot_dir) -> pandas.DataFrame` com índice `date` (diário, sem 29/fev quando a regra do protocolo mandar descartar fica a cargo de `scenarios`), colunas `PRECTOTCORR, T2M, T2M_MAX, T2M_MIN`, ausentes como NaN (nunca 0). `data.verify_manifest(snapshot_dir)` confere hashes. Falha alta e clara se o hash divergir.
- `catalog.load_catalog(config_dir) -> Catalog`; `catalog.enumerate_plans(catalog, soil, zarc_risk=0.20) -> (list[Plan], rejections)`; `Plan` imutável, com `id` estável (tupla de índices do catálogo).
- `indicators.loss_components(plan, climate_pair, params) -> (g1, g2, g3)` em [0,1]; função pura.
- `optimization.run(...) -> RunResult` (dataclass serializável em JSON): planos por política (P0, P1, P2, P1b), vetores de objetivos, Pareto, desempate, diagnósticos, `snapshot_id`, `config_hash`.
- `evaluation.evaluate(run_result, climate, protocol) -> EvalReport` recalcula as perdas a partir dos planos; não aceita escores do otimizador.
- `pipeline.run_all(inputs) -> (RunResult, EvalReport)` orquestra; o app só chama `pipeline`.

## optimization (Optimization Engineer — `catalog.py`, `indicators.py`, `optimization.py`)

Estado em 14/11 ~15h30: implementado e testado (`tests/test_catalog.py`, `test_indicators.py`, `test_optimization.py`). Assinaturas reais:

- `catalog.load_catalog(config_dir) -> Catalog` (confere o SHA-256 da tábua ZARC; duplicata de linha = erro); `catalog.load_soil(config_dir, ad_class=None, texture=None) -> Soil` (padrão `soil.toml`: AD4 + argilosa, hipótese).
- `catalog.enumerate_plans(catalog, soil, zarc_risk=0.20) -> (list[Plan], list[Rejection])`. Aceita só `zarc_risk <= 0.20` (`CatalogError` acima disso). Lista vazia = INVIÁVEL; `catalog.diagnose_infeasible(rejections)` devolve o diagnóstico (nada é relaxado).
- `Plan` imutável: `id = (c1, d1, c2, c3, d3, c4)` (índice da cultura no catálogo e ordinal cronológico do decêndio de J1/J3; pousio = último índice do catálogo), `occupations` (4 × `Occupation(window, crop, sow_t, emerge_t, harvest_t, dec)`), `signature`, `fallow_windows()`. Dia-índice `t`: 0 = 1/set, 365 dias/ano sem 29/fev, horizonte 730.
- 2ª safra (J2/J4): semeadura derivada = 1º dia ≥ colheita anterior + margem num decêndio a 20%, até 30/abr; pousio só se nenhuma 2ª safra é viável (`catalog.fallow_summary(plans)` conta os planos com pousio).
- `catalog.verify_plan(catalog, soil, plan) -> list[str]` (verificador independente das restrições; lista vazia = aprovado; é o M0). `catalog.PlanSpec`, `default_p0_spec(catalog)`, `resolve_spec(catalog, soil, spec, plans) -> (Plan | None, motivos)`, `describe_plan(catalog, plan) -> dict`, `index_plans(plans) -> {id: Plan}`.
- `indicators.load_indicator_params(config_dir, catalog) -> IndicatorParams`; `params.with_reference(ConstructionClimatology)` (mediana de e3 SÓ da construção; sem ela `loss_components` falha). `indicators.ClimatePair(P, TX, TN)` (730 dias, sem NaN; ausente = `IndicatorError`). `indicators.loss_components(plan, pair, params) -> (g1, g2, g3)` em [0,1], pura (o par memoiza ocorrências; mesma saída). `occurrence_indicators(pair, params, crop_idx, sow_t)` devolve e1..e5 e eps (para explicações no app).
- `g3`: D2 = r1 + r2 + r3 (r3 = milho depois de milho em J1→J2 **ou** J3→J4, no máximo 1), g3 = 0,5 (4 − D1)/3 + 0,5 D2/3, como em `config/indicators.toml`.
- `optimization.build_scenarios(climate, "construction" | "test") -> ScenarioSet` (`.scenarios`, `.valid`, `.summary()`, `.missing_log()`); a fatia por split é feita antes de qualquer cálculo. `construction_climatology(construction_set) -> (ConstructionClimatology, s_barra)` usa só os anos que aparecem em pares válidos da construção. Pares: construção y = 1981..2008 (28), teste y = 2010..2023 (14). Cenário inválido = qualquer NaN (PRECTOTCORR, T2M, T2M_MAX, T2M_MIN) ou dia ausente; sai para todas as políticas. < 20 válidos na construção ou < 10 no teste => `status = "DADOS_INSUFICIENTES"`.
- `optimization.run(climate, catalog, soil, params, *, snapshot_id, p0=None, weights=None, k_alternatives=6, min_hamming=2) -> RunResult`. `status` ∈ `OK | INVIAVEL | DADOS_INSUFICIENTES`; `message`; `policies` = `{P0, P1, P1b, P2}` (cada uma com `plan` (`describe_plan`), `metrics_construction` {L_sbar, mean_L, max_regret, mean_regret}, `ties`, `fallow_windows`; P0 fora do ZARC <= 20% vem com `status = "INVIAVEL_SOB_ZARC"`, `reasons` e sem `plan`, e sai do regret); `objectives` (linhas `[id, mean_g1, max_g1, mean_g2, mean_g3]`), `pareto` (`front_ids`, `equivalents`), `alternatives` (assinaturas distintas; `labels`, `reason`, `on_front`, `policy_plan_ids`), `sensitivity` (15 linhas com P1/P1b/P2 por ponto da grade), `fallow` e `fallow_usage` (quantos planos/políticas/pontos usam pousio), `rejections(_summary)`, `scenarios`, `climatology`, `snapshot_id`, `config_hash`. `RunResult.to_json()`, `to_dict(include_meta=False)`, `from_dict`, `result_hash()` (SHA-256 do JSON canônico sem `meta`; sem horários; `meta` é livre para tempos/versões).
- Funções puras reutilizáveis: `loss_matrix(comp, w)`, `regret_matrix(L)`, `select_mmr/select_exp/select_conv`, `pareto_front`, `components_tensor(plans, pairs, params)`, `q(v) = round(v, 9)`, `WEIGHT_GRID` (15 triplas em quartos; o padrão é o 5º ponto).
- Desempate (lexicográfico, depois menor ID): P2 = max R, média R; P1 = L(x, s_barra), média L; P1b = média L, max R. Médias com `math.fsum`.
- Não faz I/O. Quem chama (pipeline/evaluation) carrega `load_catalog`, `load_soil`, `load_indicator_params` e `data.load_climate`.
- Nota para o Evaluation: o teste nunca entra em `run` além da contagem de validade (portão); `build_scenarios(climate, "test")` é a fonte dos 14 pares de teste.
