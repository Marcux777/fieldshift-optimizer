# Fluxo de trabalho

Canal de coordenação: `nasa-spaceapps-2026-fieldshift`. Coordenador: Development Lead. Convoca UX/UI Designer, Critical Reviewer, VSDD Reviewer ou Requirements Engineer só se necessário.

## Responsáveis

| Entrega | Responsável |
| --- | --- |
| Snapshot NASA POWER versionado, manifesto, unidades, ausências | MLOps & Data Engineer |
| Calendário agronômico, catálogo de culturas, solo, indicadores | Methods Scientist |
| Formulação, solver, Pareto, minimax regret | Optimization Engineer |
| App Streamlit, testes, integração, hospedagem | Software Engineer |
| Protocolo, baselines, auditoria de vazamento, resultados | Evaluation & Benchmark Engineer |
| Campos em inglês, metodologia, limitações, referências, demo | Scientific Writer |
| Integração, riscos, estado | Development Lead |

## Cronograma (Brasília)

- 13/11: preparação; sem código. Estrutura do repositório criada.
- 14/11, a partir de 9h: pipeline de dados, catálogo, motor de comparação e interface mínima; escopo congelado às 14h.
- 15/11: avaliação, correções, documentação, hospedagem pública, demo. Meta de submissão 21h; limite oficial 23h59.

## Regras de entrega

- Toda entrega vem com testes automatizados e saída real de execução; relatar estado, alterações, verificações, resultados e bloqueios.
- Branch `main` até a integração; branches de trabalho seguem `<tipo>/<slug>`.
- Publicação do repositório, hospedagem e submissão só com aprovação do usuário.
