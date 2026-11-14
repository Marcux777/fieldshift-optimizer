# Preparação Software Engineer — 13/11/2026

Somente planejamento. [V] = verificado por inspeção/comando ou documentação oficial hoje; [N] = proposto, pendente ou não executado. Rigor 1/F0 para esta nota; interfaces abaixo são propostas, não contrato congelado nem implementação. Nenhuma revisão formal ou teste do app foi realizado.

## 1. Ambiente local

[V] `uv --version`: 0.12.21. `uv python list --only-installed`: Python 3.12.14 disponível em `~/.local/share/uv/python/cpython-3.12-linux-x86_64-gnu/bin/python3.12`; sistema `/usr/bin/python3`: 3.14.10. Também há interpretadores 3.11 e 3.13; não são necessários para a proposta.

| Pacote | Python 3.12.14 | Sistema 3.14.10 |
|---|---|---|
| Streamlit | [V] ausente | [V] ausente |
| pytest | [V] ausente | [V] ausente |
| numpy | [V] ausente | [V] ausente |
| pandas | [V] ausente | [V] ausente |
| matplotlib | [V] ausente | [V] ausente |
| plotly | [V] ausente | [V] ausente |

[V] Consulta somente de metadados via `importlib.metadata.version`, sem importar os pacotes: `uv run --no-project --offline --no-python-downloads --python <interpretador> python -B -c '<consulta de sys.version e metadados dos seis pacotes>'`. A primeira consulta também usou `--no-sync`; uv avisou que essa opção não tem efeito junto de `--no-project`. Nenhuma instalação/sincronização foi feita. Ausência nesses dois interpretadores não implica ausência em todos os ambientes de outros projetos.

[V] O `pyproject.toml` já existente em `/home/marcusvinicus/p/fieldshift-optimizer` declara Python >=3.12 e dependências vazias; `.venv` não existe. O repositório foi criado pelo coordenador a pedido posterior do usuário (plano, seção 7); esta tarefa apenas leu o manifesto, sem alterar esse projeto.

[N] Para 14/11, corrigido por instrução do usuário: executar Python, instalação de dependências, testes e app exclusivamente em container Docker do projeto, com Python 3.12 e uv dentro da imagem. Não criar ambiente Python no host. Adicionar Streamlit, numpy, pandas e pytest; escolher Plotly para gráficos interativos. Matplotlib só se necessário. Fixar versões resolvidas em `uv.lock`. Compatibilidade conjunta e execução ainda não verificadas. Bibliotecas estatísticas adicionais dependem do protocolo final; nenhuma dependência de solver é presumida.

[V] Docker respondeu no contexto `default`, servidor 29.9.3. Inspeção de containers, inclusive parados, não encontrou container FieldShift; busca de arquivos Compose/Dockerfile/devcontainer no projeto não encontrou definição. Não reutilizar containers de outros projetos. [N] A partir do horário autorizado: definição Compose estável do projeto, serviço proposto `app`, usuário alinhado ao host, checkout montado apenas no diretório necessário com rótulo SELinux adequado e caches de download compartilhados conforme política Docker. Criar uma vez e reutilizar com `compose exec`; se parado, iniciar o mesmo container. Instalação, pytest, lint e Streamlit rodam nele, sem ambiente paralelo no host. Porta de desenvolvimento restrita a localhost. Ainda não há container nem imagem FieldShift validados; a inspeção Docker não prova execução do app. Configuração e criação ficam para 14/11 às 9h, preservando o bloqueio de implementação.

## 2. Hospedagem e contas

- [V documental] **Primeira opção: Streamlit Community Cloud**. Exige conta Community Cloud conectada ao GitHub, acesso ao repositório, revisão/branch e arquivo de entrada definidos, dependências e snapshot disponíveis no deploy. Configurar app público; a documentação permite acesso público e informa que repositório público gera app público por padrão. [Contas e conexão](https://docs.streamlit.io/deploy/streamlit-community-cloud/get-started), [preparação do deploy](https://docs.streamlit.io/deploy/streamlit-community-cloud/deploy-your-app), [visibilidade](https://docs.streamlit.io/deploy/streamlit-community-cloud/share-your-app).
- [V documental] **Alternativa: Hugging Face Spaces público, Docker SDK com template Streamlit**. O SDK Streamlit nativo está descontinuado. A documentação atual exige plano PRO pessoal ou Team/Enterprise para criar Spaces de compute (Docker/Gradio); não tratar como alternativa gratuita garantida. Precisa conta HF, plano elegível, configuração de container/porta e dependências. Space público permite acesso ao app e ao código. [Aviso Streamlit](https://huggingface.co/docs/hub/spaces-sdks-streamlit), [planos e acesso público](https://huggingface.co/docs/hub/spaces-overview). Somente inspeção Docker local foi executada; nenhum arquivo Docker foi criado. Community Cloud continua opção de hospedagem gerenciada, mas não executa nossa imagem Docker; se a exigência abranger a hospedagem, escolher plataforma de containers, com conta e eventual custo aprovados.
- [V documental] **Repositório público: GitHub**. Precisa conta pessoal ou organização com permissão de criação; escolher dono, nome, visibilidade e licença. [Pré-requisitos oficiais](https://docs.github.com/en/repositories/creating-and-managing-repositories/creating-a-new-repository). GitHub hospeda o código; o servidor Python fica na hospedagem do app.

[N / depende do usuário] Não foram verificadas contas, sessões, permissões, planos, cotas ou disponibilidade de nomes. Criar remoto, push, conectar contas, publicar app/repositório e contratar plano exigem autorização específica; nada foi publicado. Após autorização e implementação, testar URL pública sem sessão, inclusive inicialização a frio, snapshot offline e execução completa. Acesso anônimo real, desempenho e disponibilidade ainda não verificados. Não adicionar login de juízes ao app.

## 3. Estrutura e interfaces propostas — apenas texto [N]

Aproveitar as pastas existentes; nenhum arquivo abaixo foi criado:

- `app.py`: entrada Streamlit e composição; coleta entrada, chama núcleo e apresenta resultado.
- `src/fieldshift/data.py`: adaptador de leitura/validação do snapshot e manifesto.
- `src/fieldshift/catalog.py`: culturas, janelas, ciclos, sucessões e viabilidade.
- `src/fieldshift/indicators.py`: cálculo puro dos indicadores/perda, compartilhado pelas políticas.
- `src/fieldshift/optimization.py`: enumeração viável, políticas, regret de construção e Pareto.
- `src/fieldshift/evaluation.py`: avaliação dos planos fixados nos cenários reservados.
- `config/`: catálogo, parâmetros e protocolo congelados; `data/snapshots/`: dados e manifesto; `tests/`: casos mínimos; `reports/`: resultados e registros; `docs/`: fontes e limitações.

Fluxo proposto: dados validados + catálogo → conjunto viável → otimização na construção → planos fixados → avaliação → app. Catálogo e clima são entradas distintas, não transformar dados meteorológicos em regras agronômicas.

| Fronteira | Entrada → saída proposta |
|---|---|
| Dados | snapshot + manifesto → clima diário (data, variável/unidade, ausências), proveniência e relatório de qualidade |
| Catálogo | culturas + solo + quatro janelas + restrições → planos viáveis com IDs estáveis; rejeições com motivo |
| Otimização | planos + clima de construção + configuração → planos por política, vetores de objetivos, Pareto e critérios de desempate |
| Avaliação | planos fixados + clima de teste + parâmetros ajustados na construção → métricas por plano/cenário e limitações |
| App | entradas validadas + resultados → comparação, gráficos, justificativas, fontes e mensagens de erro |

Núcleo sem rede/arquivos/Streamlit; adaptadores fazem I/O. Avaliador recalcula indicadores a partir dos planos, sem aceitar escores fornecidos pelo otimizador. IDs de snapshot/configuração acompanham resultados. Unidade, sentido de cada objetivo, formas das tabelas e regra de ausentes precisam ser acordados antes da implementação. Nada de fallback silencioso para clima sintético ou resultado pré-fabricado.

## 4. Testes críticos para 14/11 [N — planejados, não executados]

Reusar T1–T7 de `prep/evaluation-protocol.md`, lido hoje, sem alterar seu protocolo:

1. Dados: fill -999 vira ausente, nunca zero/dia seco; detectar datas duplicadas, lacunas, unidades inválidas e hash divergente; erro identifica campo/data.
2. Catálogo: rejeitar sucessão proibida e semeadura anterior à colheita; verificar limites de datas e quatro janelas completas. Comparar enumeração com contagem manual em catálogo pequeno.
3. Otimização: tabela manual de 3 planos × 3 cenários verifica escolhas, empates, regret não negativo e zero do oráculo; incluir objetivos iguais, dominância estrita e alternativas duplicadas para Pareto.
4. Vazamento: nenhum par cruza o split; perturbar teste não muda planos, normalização, limiares ou clima médio de construção.
5. Paridade/determinismo: mesmas restrições, snapshot, avaliador e pesos para todas as políticas; reexecução limpa reproduz resultado substantivo, excluindo timestamps do payload comparado.
6. App: teste focado com Streamlit AppTest, se disponível na versão escolhida, para entrada inválida, conjunto viável vazio e comparação completa; confirmar fontes, hipóteses e unidades na tela. Depois, execução real local offline pelo entrypoint com snapshot real; smoke de HTTP sozinho não comprova o fluxo.
7. Deploy autorizado: acesso anônimo e interação completa com snapshot empacotado; indisponibilidade externa deve aparecer como falha, sem inventar resultados.

Bloqueios para implementar: relógio (14/11 9h Brasília), catálogo/calendário e formulação acordados, protocolo congelado e snapshot entregue. Bloqueios para publicar: conta/permissões e autorização do usuário. Os testes de hoje se limitam à inspeção do ambiente e documentação; nenhum algoritmo/app existe como entrega desta tarefa.

## Fechamento

1. SATISFIED: ambiente consultado, ausências identificadas; instalação futura pendente.
2. SATISFIED: opções e fontes oficiais conferidas; contas/deploy não verificados.
3. SATISFIED: esqueleto textual proposto; interfaces ainda não congeladas.
4. SATISFIED: estratégia crítica registrada; execução fica para 14/11.

Autochecagem estática: proposta cobre os quatro itens, distingue evidência de intenção e preserva o bloqueio de implementação/publicação. Único arquivo de entrega escrito nesta tarefa: esta nota; sem código, instalação, commit ou alteração no repositório.
