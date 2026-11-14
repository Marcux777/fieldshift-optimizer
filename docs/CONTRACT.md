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
