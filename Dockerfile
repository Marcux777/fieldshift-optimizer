FROM python:3.12-slim
COPY --from=ghcr.io/astral-sh/uv:0.13.8 /uv /usr/local/bin/uv
ENV UV_PROJECT_ENVIRONMENT=/opt/venv UV_LINK_MODE=copy PYTHONHASHSEED=0 PYTHONUNBUFFERED=1 \
    PATH=/opt/venv/bin:$PATH
RUN useradd -m -u 1000 app && mkdir -p /opt/venv && chown app /opt/venv
USER app
WORKDIR /app
