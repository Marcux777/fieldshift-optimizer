# NASA Space Apps 2026 — FieldShift Optimizer (briefing completo)

Canal: `nasa-spaceapps-2026-fieldshift`. Coordenador: Development Lead. Prioridade alta, prazo inadiável.

## Competição
- NASA Space Apps Challenge 2026, Universal Event (100% remoto).
- Desafio: Field Shift: Adapting Farms with NASA Data
  https://www.spaceappschallenge.org/2026/challenges/field-shift-adapting-farms-with-nasa-data/
- Hackathon de software, dados e ciência aplicada, com protótipo funcional.
- Datas: 14 e 15/11/2026. Início autorizado de desenvolvimento: 14/11 às 9h.
- Submissão final: 15/11 às 23h59 (Brasília). Meta interna: concluir até 21h.
- Respeitar as regras oficiais. Planejamento e pesquisa podem ocorrer antes; a implementação específica só começa no horário autorizado.

## Projeto
**FieldShift Optimizer** — Robust Crop Rotation Decisions Under Climate Uncertainty.

Problema: o agricultor escolhe sequências de culturas sob incerteza de chuva, períodos secos, solo e limitações. Uma rotação ótima no clima médio pode ir mal em anos secos.

Solução: app web de apoio à decisão que:
1. Recebe características da propriedade, culturas disponíveis e prioridades.
2. Integra observações climáticas autênticas da NASA e informações agronômicas verificáveis.
3. Gera sequências temporal e agronomicamente viáveis.
4. Compara a rotação atual com alternativas otimizadas.
5. Faz otimização multiobjetivo e identifica soluções não dominadas.
6. Compara decisões por clima médio vs minimização do arrependimento máximo (minimax regret).
7. Avalia estabilidade em cenários climáticos históricos distintos.
8. Explica fatores e condições em que a recomendação muda.
9. Exibe fontes, hipóteses, incertezas e limitações.

Pergunta central: "Qual rotação agrícola representa uma escolha mais defensável quando não sabemos quais condições climáticas ocorrerão?"
Mostrar o custo dos compromissos, quando uma decisão deixa de ser preferível e a evidência de cada comparação.
Não alegar originalidade científica inédita nem superioridade sem validação.

## Escopo do MVP
- Região: Rio Verde, GO. Uma propriedade de sequeiro. Horizonte de dois anos agrícolas.
- Quatro janelas de cultivo, com datas e restrições verificadas.
- Catálogo pequeno de culturas agronomicamente fundamentadas.
- Dados históricos reais da NASA em snapshot versionado.
- Solo informado ou de fontes identificadas.
- Indicadores de exposição a períodos secos; indicadores indiretos de diversidade e continuidade de cobertura do solo.
- Otimização multiobjetivo e fronteira de Pareto.
- Comparação: plano atual, otimização convencional e minimax regret.
- Avaliação retrospectiva, com separação entre dados de construção e teste.
- Interface web simples, preferencialmente Streamlit.
- Não prometer produtividade, lucro, economia de água ou recuperação do solo sem evidência suficiente.

## Equipe (membros do canal)
1. Development Lead (coordenador): execução geral, decomposição, integração, riscos, prioridades, entrega funcional no prazo.
2. Optimization Engineer: modelagem, restrições, Pareto, minimax regret, alternativas distintas, correção de algoritmos.
3. MLOps & Data Engineer: dados NASA, datasets, unidades e ausências, snapshots versionados, rastreabilidade, execução offline.
4. Methods Scientist: fundamentação dos indicadores, validação das hipóteses agronômicas, calendários, evitar conclusões não sustentadas.
5. Software Engineer: aplicação, integração, interface, tratamento de erros, testes.
6. Evaluation & Benchmark Engineer: baselines, experimentos comparativos, validação dos indicadores, robustez, vieses e vazamento temporal.
7. Scientific Writer: documentação técnica e científica, metodologia, limitações, referências, narrativa e materiais da submissão.

Sob demanda (fora do canal; só o Development Lead convoca): UX/UI Designer, Critical Reviewer, VSDD Reviewer, Requirements Engineer. Evitar muitos agentes simultâneos.

## Organização
O canal é o centro de coordenação. O Development Lead mantém: escopo aprovado e requisitos essenciais; responsável por cada entrega; dependências e impedimentos; estado real da implementação; evidências de testes e validação; pendências da submissão.
Especialistas colaboram em entregas integráveis, sem módulos incompatíveis nem pesquisa repetida. Sem discussão indefinida, burocracia ou expansão de escopo.

## Planejamento
- **13/11 (preparação autorizada):** revisar desafio, recursos oficiais, requisitos, referências e riscos. Confirmar quais dados NASA satisfazem a elegibilidade. Não iniciar a implementação específica.
- **14/11 (desenvolvimento, a partir das 9h):** criar o projeto; pipeline de dados, catálogo agronômico, motor de comparação e interface mínima. Priorizar fluxo completo e reproduzível.
- **15/11 (validação e submissão):** executar comparações, corrigir defeitos, documentar limitações, preparar demonstração e submeter cedo (meta 21h).

## Critérios de aceite
1. Aplicação funcional e demonstrável.
2. Fonte NASA elegível para o desafio.
3. Fontes e parâmetros dos dados documentados.
4. Rotações respeitam as restrições obrigatórias.
5. As três políticas comparadas em condições equivalentes.
6. Avaliação retrospectiva sem usar dados de teste para escolher parâmetros ou soluções.
7. Recomendações com justificativas verificáveis.
8. Testes críticos executados e resultados registrados.
9. Documentação, repositório e demonstração acessíveis.
10. Submissão oficial confirmada.

Resultados negativos ou inconclusivos são aceitáveis; resultados inventados não.

## Instrução inicial ao coordenador
Development Lead, organize a equipe com base neste briefing. Antes do início oficial, identifique só dependências críticas, responsáveis, riscos e ordem de execução. Durante a competição, priorize integração e validação. Não presuma funcionalidade concluída sem evidência de execução e testes. Ao reportar: estado, alterações, verificações executadas, resultados e bloqueios.
