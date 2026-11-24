# REVIEW.md — Lições Estruturais e Travas Mecânicas (FieldShift)

Registro auditável de falhas convertidas em controles executáveis no CI e no pre-commit (`scripts/check-fieldshift-structure` e `scripts/verify-fieldshift`).

| Data | Agente | Sintoma | Trava mecânica adicionada |
| --- | --- | --- | --- |
| 2026-11-23 | Software Engineer | Risco de IO/env no núcleo científico puro (`numeric`/`model`/`eval`) ou violação do DAG de crates | `scripts/check-fieldshift-structure --check C1` |
| 2026-11-23 | Software Engineer | Risco de replay de `reports/` ou `fixtures/` em código produtivo ou mutação de snapshot congelado | `scripts/check-fieldshift-structure --check C2` |
| 2026-11-23 | Software Engineer | Crescimento monolítico de arquivos (>400 linhas) e funções (>60 linhas) acima do baseline `891818f` | `scripts/check-fieldshift-structure --check C3` |
| 2026-11-23 | Software Engineer | Uso de `.unwrap()` / `.expect()` em código de produção Rust ou `: any` em TypeScript | `scripts/check-fieldshift-structure --check C4` |
| 2026-11-23 | Software Engineer | Uso de `page.waitForTimeout` / `sleep` arbitrário em testes ou `TODO` sem referência | `scripts/check-fieldshift-structure --check C5` |

```vsdd-lesson
schema_version: vsdd-lesson/1
date: "2026-11-23"
category: pure-core-and-dune-boundaries
rule: "O núcleo científico puro e o frontend respeitam as 5 classes da camada Dune (C1-C5) antes de qualquer revisão."
evidence: "RUST_CONTRACT.md e histórico de commits 1ea3fb1, 5cc3a5a e 011b7be"
approved_on: "2026-11-23"
mechanism: lint
control:
  path: scripts/check-fieldshift-structure
  command: "python3 scripts/check-fieldshift-structure"
  regression_test: "python3 scripts/check-fieldshift-structure --self-test"
negative_control:
  command: "python3 scripts/check-fieldshift-structure --negative-test"
  description: "Executa fixtures negativas para C1-C5 e exige que todas sejam reprovadas"
state: implantado
layer: projeto
```
