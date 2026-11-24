# Escudo Anti-Regressão do FieldShift Optimizer

Documentação operacional da Fase 1 (Escudo Anti-Regressão) sobre a revisão base `891818f585361abeb844a64b36e45231ed3c8c76`.

## 1. Componentes

1. **Mapa de funcionalidades ([feature-map.yaml](../feature-map.yaml)):**
   Define o comando real de build/start, endpoints (`/v1/health`, `/v1/run`, `/v1/evaluate`), as 6 abas da interface e os 3 fluxos verificáveis (`main`, `invalid-weights`, `exploratory-soil`).
2. **Verificador de produto ([scripts/verify-fieldshift](../scripts/verify-fieldshift) e [.agents/skills/verify-fieldshift/SKILL.md](../.agents/skills/verify-fieldshift/SKILL.md)):**
   Sobe uma instância local limpa da API Rust servindo o build estático SvelteKit em porta efêmera livre, executa Playwright real (Chromium desktop `1280x800` e mobile `390x844`), grava evidências completas em `.vsdd/evidence/<mission-id>/` e encerra todos os processos filhos com `SIGTERM`/`SIGKILL`.
   - Códigos de saída: `0` (`PASS`), `1` (`FAIL`), `2` (uso inválido), `3` (`BLOCKED` quando falta pré-requisito externo ou snapshot obrigatório).
3. **Camada Dune ([scripts/check-fieldshift-structure](../scripts/check-fieldshift-structure)):**
   Checador determinístico executado no pre-commit e no CI que bloqueia 5 classes de regressão:
   - **C1 (Fronteiras de arquitetura e núcleo puro sem IO):** `fieldshift-numeric`, `fieldshift-model` e `fieldshift-eval` não acessam filesystem, rede, variáveis de ambiente ou subprocessos em código de produção, e respeitam o DAG de crates (`numeric -> ∅`, `io -> numeric`, `model -> numeric`, `eval -> numeric/model`, `cli/api -> todos`). No frontend, `src/lib/api/` não importa componentes/rotas e `domain/presentation/insights` não importam runtime de rede/DOM.
   - **C2 (Proibição de replay de `reports/` ou `fixtures/` no produto e proteção de arquivos congelados):** código de produção nunca lê `reports/` nem `fixtures/`; arquivos congelados (`reports/FROZEN_CONFIG.json`, `data/snapshots/v1.0.0/*`) não são alterados.
   - **C3 (Teto de complexidade com ratchet da base `891818f`):** arquivos novos ≤ 400 linhas; funções novas ≤ 60 linhas; outliers existentes na base `891818f` travados no seu tamanho exato (não podem crescer).
   - **C4 (Sem `unwrap()`/`expect()` em produção Rust nem `: any` em TypeScript):** proibido fora de `#[cfg(test)]` salvo exceção explícita documentada.
   - **C5 (Sem `sleep`/`waitForTimeout` arbitrário em testes nem `TODO` sem issue):** proíbe `page.waitForTimeout`, `time.sleep` e `thread::sleep` em testes e `TODO` sem referência `TODO(#N)` ou `TODO(issue)`.

## 2. Calibração Obrigatória (Fixtures A, B e C)

- **Fixture A (`--fixture A`):** aplica mutação isolada em cópia de trabalho trocando `Frozen evaluation outcome: NEGATIVE` por `Frozen evaluation outcome: POSITIVE` em `src/lib/api/client.ts`, reconstrói o bundle estático na cópia e executa o fluxo `main` real. A asserção do Playwright no banner falha → retorna `1` (`FAIL`).
- **Fixture B (`--fixture B`):** executa a árvore limpa com snapshots e configuração originais → retorna `0` (`PASS`) com screenshots, traces e hashes SHA-256 conferidos.
- **Fixture C (`--fixture C`):** aponta `FIELDSHIFT_ROOT` para um diretório temporário que preserva `config/` mas omite `data/snapshots/v1.0.0/` → retorna `3` (`BLOCKED`).
