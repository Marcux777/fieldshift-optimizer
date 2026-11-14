# FieldShift Optimizer

Robust Crop Rotation Decisions Under Climate Uncertainty. Projeto para o desafio [Field Shift: Adapting Farms with NASA Data](https://www.spaceappschallenge.org/2026/challenges/adapting-farms-with-nasa-data/) do NASA Space Apps Challenge 2026.

## Estado

Estrutura criada em 13/11/2026. Não há código nem dados ainda: o desenvolvimento começa em 14/11/2026 às 9h (Brasília). Nada aqui está implementado ou validado.

## Objetivo (MVP)

Apoio à decisão, que não substitui o agricultor nem a assistência agronômica local:

- Rio Verde (GO), propriedade de sequeiro, 2 anos agrícolas, 4 janelas de cultivo.
- Dados históricos reais da NASA (NASA POWER) em snapshot versionado.
- Otimização multiobjetivo com fronteira de Pareto.
- Comparação entre plano atual, otimização convencional (clima médio) e minimax regret.
- Avaliação retrospectiva com separação entre dados de construção e de teste.
- Interface web simples em Streamlit, com fontes, hipóteses e limitações visíveis.

## Documentação

- [`AGENTS.md`](AGENTS.md): regras e estrutura para agentes.
- [`docs/DECISIONS.md`](docs/DECISIONS.md): decisões do cliente e escopo técnico congelado.
- [`docs/planning/BRIEFING.md`](docs/planning/BRIEFING.md): escopo, equipe e critérios de aceite.
- [`docs/HACKATHON_RULES.md`](docs/HACKATHON_RULES.md): regras oficiais verificadas.
- [`docs/WORKFLOW.md`](docs/WORKFLOW.md): cronograma e responsáveis.

## Licença

MIT para o código (`LICENSE`).
