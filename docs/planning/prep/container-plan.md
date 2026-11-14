# Plano de container — FieldShift (MLOps & Data Engineer, 13/11/2026)

Somente planejamento. Nenhum arquivo foi criado no repositório; o desenvolvimento começa 14/11 às 9h (Brasília).
[V] = verificado hoje; [N] = proposto, não executado.

## Verificado hoje
- [V] Docker 29.9.3 e Podman 5.8.9 instalados; o usuário está no grupo `docker`.
- [V] `python:3.12-slim` baixada (178 MB): Python 3.12.16 roda no container.
- [V] `ghcr.io/astral-sh/uv:latest`: uv 0.13.8 roda.
- [V] O host não tem Streamlit, pytest, numpy, pandas, plotly (nota do Software Engineer). O container resolve isso sem instalar nada no host.

## Proposta [N]
- Uma imagem só, `fieldshift:dev`, base `python:3.12-slim`, uv copiado de `ghcr.io/astral-sh/uv` (multi-stage `COPY --from`).
- Dependências por `uv sync --frozen` a partir de `pyproject.toml` + `uv.lock` (camada separada, cache de build). Sem `uv.lock`, sem build.
- Usuário não-root; `PYTHONHASHSEED` e semente fixos; `data/snapshots/` entra na imagem (app roda offline com o snapshot versionado).
- `HEALTHCHECK` em `/_stcore/health`; porta 8501; `CMD streamlit run app.py --server.address=0.0.0.0`.
- `.dockerignore`: `.git`, `.venv`, `__pycache__`, `reports/` temporários.
- Alvos (via `docker compose` ou `make`):
  - `test`: `docker run --rm fieldshift:dev uv run pytest` (T1–T7 do protocolo de avaliação).
  - `run`: sobe o app em `localhost:8501`.
  - `repro`: roda a avaliação duas vezes e compara o hash do resultado (idempotência / reprodutibilidade).
- Rascunho do Dockerfile (texto, para criar em 14/11):

```dockerfile
FROM python:3.12-slim AS base
COPY --from=ghcr.io/astral-sh/uv:0.13.8 /uv /usr/local/bin/uv
ENV UV_COMPILE_BYTECODE=1 UV_LINK_MODE=copy PYTHONHASHSEED=0 PYTHONUNBUFFERED=1
WORKDIR /app
COPY pyproject.toml uv.lock ./
RUN uv sync --frozen --no-install-project
COPY . .
RUN uv sync --frozen && useradd -m app && chown -R app /app
USER app
EXPOSE 8501
HEALTHCHECK CMD python -c "import urllib.request as u;u.urlopen('http://localhost:8501/_stcore/health')"
CMD ["uv","run","streamlit","run","app.py","--server.address=0.0.0.0","--server.port=8501"]
```

## Impacto na hospedagem
- Streamlit Community Cloud **não usa Docker**: ele instala a partir do repositório (`requirements`/`pyproject`). O container valida o mesmo ambiente localmente, mas não é o que roda lá.
- Hugging Face Spaces com Docker SDK usaria esta mesma imagem, mas a nota do Software Engineer registra exigência de plano elegível. Não verificado por mim.
- Decisão do usuário: o container é para (a) desenvolvimento/testes reproduzíveis, ou (b) também para publicar? Se (b), a hospedagem precisa aceitar Docker.

## Medições obrigatórias (após o código, antes da revisão VSDD)
Tempo de build frio e com cache, tamanho da imagem, tempo de inicialização até o health OK, pico de memória do app (`docker stats`) e latência da avaliação completa. Nada medido ainda, pois não há código.

## Pendências
- Aguardar 14/11 9h para criar `Dockerfile`, `.dockerignore`, `uv.lock` no repositório.
- Definir com o Software Engineer a lista final de dependências (Streamlit, numpy, pandas, pytest, Plotly).
