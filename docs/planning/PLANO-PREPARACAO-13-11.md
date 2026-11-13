# FieldShift Optimizer — plano de preparação (13/11/2026)

Coordenador: Development Lead. Só preparação hoje; nenhum projeto ou código do FieldShift antes de 14/11 9h (Brasília).
Marcas: [V] verificado hoje com fonte; [N] não verificado.

## 1. Regras oficiais verificadas (spaceappschallenge.org, lidas em 13/11)
- [V] URL correta do desafio: https://www.spaceappschallenge.org/2026/challenges/adapting-farms-with-nasa-data/ (a URL do briefing com `field-shift-` no slug dá 404).
- [V] Desafio: ferramenta de apoio à decisão com observações NASA + solo local + características de culturas + prioridades do agricultor. "Pode comparar ao menos três estratégias de rotação, incluindo a existente". Deve apoiar, não substituir o julgamento do agricultor; explicar evidência, lacunas e riscos por estratégia. Nada é obrigatório ("may").
- [V] Dados NASA citados: SMAP, GPM/IMERG, Landsat, MODIS/VIIRS, NASA POWER, SRTM, ECOSTRESS, GRACE/GRACE-FO, GEDI, NLDAS/GLDAS. Elegibilidade global: "pelo menos um dado ou recurso NASA" (Guia de Submissão). NASA POWER é elegível.
- [V] Recursos não-NASA listados são centrados nos EUA (CDL, WSS, SSURGO, PRISM, NAIP, NASS). Não cobrem Rio Verde (GO). Listados e utilizáveis fora dos EUA: FAO GAEZ, ERA5, Copernicus Sentinel-2, OpenET (cobertura a confirmar). Solo/culturas de Rio Verde exigem fonte brasileira (ex.: Embrapa, Conab, ZARC) [N, a pesquisar pelo Methods Scientist].
- [V] Agência parceira: Brazilian Space Agency está na lista; uso de dados de parceiros é "altamente encorajado", não obrigatório.
- [V] Acesso ao NASA POWER: chamada diária pontual (lat -17.80, lon -50.93, 3 dias de 2024) retornou HTTP 200, fonte MERRA2, PRECTOTCORR mm/dia, T2M °C, fill_value -999. Teste só de leitura, nada gravado. Coordenadas de Rio Verde são aproximadas [N].
- [V] Submissão: abre 14/11 9h, fecha 15/11 23h59, no fuso do dono da equipe (Universal Event). Aba "Project" só aparece a partir de 14/11 9h. Reenvio permitido até o fim; vale o último salvo.
- [V] Exigências: equipe Universal com ≥2 pessoas e ≤6, todas registradas no mesmo evento; campos todos preenchidos, em inglês; demo ≤30 s de vídeo ou ≤7 slides (inclusive título), em link público sem login (senão inelegível ao julgamento global); link público do projeto final; campo NASA Data; lista de todas as referências (código, texto, imagens); caixas "Terms" e "Confirm All Work is Original"; sem imagem/voz de menores de 18 anos; sem marca NASA em conteúdo gerado.
- [V] Uso de SI: permitido, mas "SI não deve ser usada para criar a ideia de como resolver o desafio"; é preciso indicar onde SI foi usada (código e dados: no texto descritivo e nos metadados; imagens/vídeos: marca d'água visível). Julgamento: Impact, Creativity, Validity, Relevance, Presentation.
- [N] Não li os Termos e Condições (/legal/) nem o PDF do Guia de Julgamento por completo.
- [N] Não verifiquei: se a equipe está registrada, se há ≥2 humanos, quem é o dono da equipe e o fuso dele.

## 2. Dependências críticas
1. Equipe registrada e elegível (≥2 pessoas) e login do usuário para submeter. Bloqueio do usuário, fora do controle do canal.
2. Fonte NASA com acesso sem credencial: NASA POWER (verificado). IMERG, SMAP etc. exigem login Earthdata [N]; ficam fora do MVP salvo decisão contrária.
3. Calendário agronômico + catálogo de culturas de Rio Verde com fonte citável (Methods Scientist).
4. Fonte de solo identificada, ou solo informado pelo usuário como parâmetro explícito.
5. Formulação fechada antes de codar: janelas, restrições, objetivos, regret, definição de Pareto (Optimization).
6. Protocolo de avaliação com split e regras de vazamento congelado antes de ver resultados (Evaluation).
7. Hospedagem pública do app e do repositório, sem login (Software Engineer + usuário aprova publicar).
8. Demo (7 slides ou 30 s) e textos EN dos campos (Scientific Writer).

## 3. Responsáveis
| Entrega | Responsável |
|---|---|
| Snapshot NASA POWER versionado, manifesto, unidades, ausências | MLOps & Data Engineer |
| Calendário, catálogo de culturas, solo, indicadores | Methods Scientist |
| Formulação, solver, Pareto, minimax regret | Optimization Engineer |
| App Streamlit, testes, integração, hospedagem | Software Engineer |
| Protocolo, baselines, auditoria de vazamento, resultados | Evaluation & Benchmark Engineer |
| Campos EN, metodologia, limitações, referências, demo | Scientific Writer |
| Integração, riscos, estado, convocações | Development Lead |

## 4. Riscos principais
1. Fonte agronômica de Rio Verde frágil → catálogo pequeno, parâmetros marcados como hipótese, sem promessa de produtividade/lucro.
2. Só 2 anos × 4 janelas deixa o espaço de busca pequeno → enumeração exaustiva basta; não superdimensionar solver.
3. Vazamento temporal na avaliação retrospectiva → split por anos congelado antes de rodar.
4. Resultado negativo (minimax regret não supera) → reportar como está.
5. Hospedagem/link público e filtro de rede do julgamento → decidir 14/11 cedo; considerar o formulário FortiGuard.
6. Uso de SI e originalidade: ideia é do usuário; documentar uso de SI em código/dados.
7. Equipe/registro/fuso da submissão não confirmados.
8. Prazo curto: congelar escopo às 14h de 14/11; meta de submissão 15/11 21h.

## 5. Ordem de execução
14/11 (a partir de 9h)
- 9h: criar repositório/projeto e estrutura; congelar escopo e interfaces (Dev Lead + Software Engineer).
- 9h–12h: snapshot POWER + manifesto (MLOps); calendário/catálogo congelados (Methods); formulação congelada (Optimization); protocolo congelado (Evaluation) — em paralelo.
- 12h–18h: motor de otimização + três políticas sobre o mesmo snapshot; interface mínima; testes críticos.
- 18h–22h: fluxo completo ponta a ponta executando; primeira execução de avaliação.
15/11
- 8h–14h: rodar avaliação retrospectiva, corrigir defeitos, registrar resultados (incluindo negativos).
- 14h–18h: documentação, limitações, referências, hospedagem pública, demo.
- 18h–21h: revisão final (VSDD Reviewer/Critical Reviewer só se necessário), pedido de aprovação ao usuário.
- Meta 21h: submissão, só com aprovação do usuário. Limite oficial 23h59.

## 6. Pedidos de preparação enviados (sem implementação)
Resultados esperados em /home/marcusvinicus/OpenBot/Shared/nasa-spaceapps-2026-fieldshift/prep/<papel>.md
