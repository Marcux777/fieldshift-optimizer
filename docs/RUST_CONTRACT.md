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
_(MLOps)_
### io
_(MLOps)_
### model
_(Optimization)_
### eval
_(Evaluation)_
### api
_(Software Engineer: contrato OpenAPI em `backend/crates/fieldshift-api/openapi.json`)_
