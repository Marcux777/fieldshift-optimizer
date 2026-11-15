# Progress (Development Lead)

Estado em 14/11/2026 ~21h (Brasília); linhas de deploy e testes atualizadas após a migração para Rust + SvelteKit. Fonte de verdade: commits, `reports/` e a saída dos testes.

| Item | Estado | Evidência |
| --- | --- | --- |
| Snapshot NASA POWER (célula -18,0/-50,625, 1981–2025) | Feito | `data/snapshots/power-daily-2.10.3-lat-18_lon-50.625-46c276253b44/manifest.json`, 26 testes |
| Catálogo, solo, indicadores, ZARC ≤20% | Feito (parâmetros agronômicos = hipóteses [N]) | `config/`, `config/zarc/VERIFICATION.md` |
| Motor (P0, P1, P2, P1b, Pareto, 15 pesos) | Feito | `|X|`=576 (AD4), run 0,39 s, hash determinístico |
| Avaliação retrospectiva | Feito; desfecho NEGATIVO | `reports/eval_runs.md` |
| App web Rust + SvelteKit | Feito; publicado no Render Free (E10). O app Streamlit original segue no repositório só como implementação de referência | https://fieldshift-optimizer.onrender.com, `docs/RUST_MIGRATION.md`, `docs/HOSTING.md` |
| App Streamlit (referência) | Feito, local; não é publicado | `tests/test_app.py`, `docs/assets/` |
| Documentação em inglês | Feita; faltam URLs, equipe, mídia SI, demo | `README.md`, `docs/SUBMISSION.md` |
| Testes | Python: `170 passed`, `ruff` limpo (contêiner `fieldshift-dev`). Rust: `cargo test --workspace` com paridade bit a bit (hashes `b4370332…` e `504c28e8…`); frontend: Vitest + Playwright | `docs/RUN.md`, CI `rust-migration.yml` |
| Revisão independente | VSDD: APROVADO em `bda6939` (re-revisão do delta; só achados menores); Critical Reviewer: COM RESSALVAS, aplicadas | commits `da19828`, `4be59f1`, `5bd8846` |
| Repositório público | Publicado em `bda6939`; público, licença MIT, clone anônimo e instalação verificados | https://github.com/Marcux777/fieldshift-optimizer |
| Deploy | Feito: Render Free (Rust + SvelteKit) | https://fieldshift-optimizer.onrender.com |
| Demo, equipe, mídia SI, submissão | PENDENTE: ação do usuário | — |
