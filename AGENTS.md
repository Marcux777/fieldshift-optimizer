# Regras comuns dos agentes

Fonte comum de instruções do projeto para Codex, Claude e outros agentes. `CLAUDE.md` a importa com `@AGENTS.md`; não mantenha cópias divergentes. Continuam aplicáveis as instruções globais do ambiente e a autorização do usuário.

FieldShift Optimizer: app web de apoio à decisão (API Rust + frontend SvelteKit, publicado no Render Free; o app Streamlit original fica no repositório só como implementação de referência dos testes diferenciais) para rotação de culturas sob incerteza climática, para o desafio "Field Shift: Adapting Farms with NASA Data" do NASA Space Apps 2026. Rio Verde (GO), sequeiro, 2 anos agrícolas, 4 janelas de cultivo, dados NASA em snapshot versionado. Compara plano atual, otimização convencional (clima médio) e minimax regret, com avaliação retrospectiva sem vazamento.

## Retomada do trabalho

1. Leia `README.md` e `docs/WORKFLOW.md` para estado, responsáveis e cronograma.
2. Confira checkout, branch e alterações existentes antes de editar.
3. Leia `docs/planning/BRIEFING.md` (escopo e critérios de aceite) e `docs/HACKATHON_RULES.md` (regras oficiais verificadas).
4. Defina papel, caminhos de trabalho, resultado esperado e verificação.

## Fontes de referência

| Assunto | Fonte |
| --- | --- |
| Decisões do cliente, escopo técnico congelado | `docs/DECISIONS.md` |
| Escopo, critérios de aceite, equipe | `docs/planning/BRIEFING.md` |
| Regras oficiais, submissão, uso de SI | `docs/HACKATHON_RULES.md` |
| Fluxo, cronograma, responsáveis | `docs/WORKFLOW.md` |
| Plano de preparação (13/11), riscos, ordem de execução | `docs/planning/PLANO-PREPARACAO-13-11.md` |
| Fontes e parâmetros de dados | `docs/DATA_SOURCES.md` |
| Protocolo de avaliação | `docs/EVALUATION_PROTOCOL.md` |
| Mapa de funcionalidades e fluxos verificáveis | `feature-map.yaml` |
| Escudo anti-regressão (camada Dune + `verify-fieldshift`) | `docs/REGRESSION_SHIELD.md` |
| Lições estruturais e travas mecânicas | `REVIEW.md` |

## Escudo Anti-Regressão (Comandos Obrigatórios Antes da Revisão)

- Checagem arquitetural determinística (Camada Dune C1–C5 + pre-commit): `python3 scripts/check-fieldshift-structure --self-test`
- Verificação real fim a fim (API Rust + build SvelteKit + Playwright desktop/mobile): `scripts/verify-fieldshift --mission <mission-id> --flows main,invalid-weights,exploratory-soil`
- Calibração das 3 fixtures (A→1 FAIL, B→0 PASS, C→3 BLOCKED): `scripts/verify-fieldshift --mission <id> --fixture A|B|C`

## Estrutura

- `src/`: código do app e do motor (a partir de 14/11 às 9h).
- `tests/`: testes `tests/test_<feature>.py`, funções `test_<comportamento>()`.
- `config/`: parâmetros declarativos (catálogo, janelas, cenários).
- `data/snapshots/<versão>/`: snapshot NASA versionado, com manifesto (versionado no Git). `data/raw/` e `data/cache/` ficam fora do Git.
- `reports/`: resultados registrados de execuções e avaliações.
- `docs/`: documentação, regras oficiais e planejamento.

## Comandos

Escopo técnico congelado em `docs/DECISIONS.md`; leia antes de qualquer mudança. A implementação Python de referência (instalação, testes, lint e app Streamlit) roda em container Docker do projeto (Python 3.12 e uv na imagem), sem ambiente Python no host, a pedido do usuário em 13/11. O app publicado é o Rust + SvelteKit: um contêiner único (`backend/Dockerfile`) no Render Free; Rust e Node rodam no host, com o toolchain fixado em `rust-toolchain.toml` (`docs/RUN.md`, `docs/HOSTING.md`). Antes do primeiro comando ou arquivo Docker, ler `~/.codex/instructions/docker.md`. Ambiente e comandos: `docs/CONTRACT.md` (contêiner `fieldshift-dev`, `docker compose exec -T dev <cmd>`). Divisão de arquivos entre agentes: `docs/CONTRACT.md`.

## Regras do projeto

- Não prometer produtividade, lucro, economia de água nem recuperação do solo sem evidência; não alegar ineditismo nem superioridade sem validação.
- Dados de teste não escolhem parâmetros nem soluções. Resultados negativos ou inconclusivos são aceitáveis; inventados, não.
- Nada é "concluído" sem execução e testes registrados.
- Código ou dado gerado com SI é declarado no texto descritivo e nos metadados (regra oficial do Space Apps); conteúdo gerado não usa marca NASA.
- Toda referência usada (código, texto, imagens, dados) entra na lista de referências da submissão.
- Segredos (login Earthdata, tokens) ficam fora do Git.
- Publicar o repositório, hospedar o app e submeter ao Space Apps exigem aprovação explícita do usuário.

## Commits

Mensagens curtas no padrão Conventional Commits (`feat:`, `fix:`, `docs:`, `test:`, `chore:`), no imperativo e no escopo de um comportamento ou artefato.
