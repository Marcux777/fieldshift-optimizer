# Registro de execuções da avaliação (protocolo v1.1)

Todas as execuções que tocam o teste, inclusive falhas e reexecuções.

| run_id | início (UTC) | estado | desfecho | eval result_hash | run_result_hash | config congelada | commit (dirty) |
|---|---|---|---|---|---|---|---|
| 20261114T181122Z | 2026-11-14T18:11:22Z | ok | NEGATIVO | `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18` | `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66` | `067e569ec333` (igual) | `indispo` (True) |
| 20261114T181200Z | 2026-11-14T18:12:00Z | ok | NEGATIVO | `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18` | `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66` | `067e569ec333` (igual) | `6661c45` (None) |

**Nota de rastreabilidade (I-1, revisão VSDD, registrada em 2026-11-14 antes da reexecução abaixo):** os runs `20261114T181122Z` e `20261114T181200Z` rodaram com a árvore de trabalho sem commit (o avaliador só foi commitado em `3b02974`). O campo `commit` dos manifestos aponta o commit-pai `6661c45` (no 1º run, "indisponivel"), e `dirty` ficou desconhecido porque o contêiner não tem `git`. Os manifestos antigos não foram reescritos. A reexecução seguinte roda em HEAD `40bad6d` com `git status --porcelain` vazio no host em todo o código (`src/`, `scripts/`, `config/`, `data/`, `tests/`); só esta nota em `reports/eval_runs.md` estava alterada.

| run_id | início (UTC) | estado | desfecho | eval result_hash | run_result_hash | config congelada | commit (dirty) |
|---|---|---|---|---|---|---|---|
| 20261114T184118Z | 2026-11-14T18:41:18Z | ok | NEGATIVO | `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18` | `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66` | `067e569ec333` (igual) | `40bad6d` (None) |

**Mudança no avaliador (VSDD M-1 e M-2), registrada antes da execução de confirmação abaixo:** (M-1) o cache de `IndicatorLossModel` usava `id(scenario)` na chave, e houve 1 colisão real no s̄ do leave-one-out. Agora ele guarda o próprio cenário junto do par e confere a identidade. (M-2) a validade do cenário passa a exigir as 4 variáveis (`PRECTOTCORR`, `T2M`, `T2M_MAX`, `T2M_MIN`), conforme o protocolo v1.1 §1.3; antes `T2M` não era checado. O snapshot tem 0 ausentes, então nenhum número deve mudar. O hash da configuração congelada (`067e569e…`) não muda: ele cobre protocolo, otimizador, snapshot, solo, pesos, grade, P0, seed e ε, mas não o código do avaliador. Esta execução roda com essas correções ainda sem commit sobre `40bad6d` e serve só para confirmar a igualdade dos hashes.

| run_id | início (UTC) | estado | desfecho | eval result_hash | run_result_hash | config congelada | commit (dirty) |
|---|---|---|---|---|---|---|---|
| 20261114T184243Z | 2026-11-14T18:42:43Z | ok | NEGATIVO | `504c28e85e54cbd4e9470e8a423499299dcbed643cb9b2ff8aa461873e134c18` | `b43703326690e6d0ddd2be48641fbdb9ba46908810ebfca0c8c90a379c79fd66` | `067e569ec333` (igual) | `40bad6d` (None) |
