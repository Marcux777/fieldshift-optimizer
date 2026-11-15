# AI-assisted (OpenBot agent)
# Build context: repository root. Runtime has Rust API + Svelte static assets, no Python/reports.
FROM node:22-bookworm-slim AS frontend
WORKDIR /build/frontend
COPY frontend/package*.json ./
RUN --mount=type=cache,id=fieldshift-npm,target=/root/.npm \
    if [ -f package-lock.json ]; then npm ci --no-audit --no-fund; else npm install --no-audit --no-fund --no-package-lock; fi
COPY fixtures/reference/golden_eval_report.json fixtures/reference/golden_run_result.json /build/fixtures/reference/
COPY frontend/ ./
RUN npm run check && npm run build

FROM rust:1.98-bookworm AS backend
WORKDIR /build/backend
# Same toolchain as local builds and CI (rustup picks it up from the parent directory).
COPY rust-toolchain.toml /build/rust-toolchain.toml
COPY backend/Cargo.toml backend/Cargo.lock ./
COPY backend/crates ./crates
RUN --mount=type=cache,id=cargo-registry,target=/usr/local/cargo/registry \
    --mount=type=cache,id=cargo-git,target=/usr/local/cargo/git \
    cargo build --locked --release -p fieldshift-api -p fieldshift-cli

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=backend /build/backend/target/release/fieldshift-api /usr/local/bin/fieldshift-api
COPY --from=backend /build/backend/target/release/fieldshift-cli /usr/local/bin/fieldshift-cli
COPY --from=frontend /build/frontend/build /app/frontend/build
COPY config/ /app/config/
COPY data/snapshots/ /app/data/snapshots/
COPY docs/EVALUATION_PROTOCOL.md /app/docs/EVALUATION_PROTOCOL.md
COPY docs/LIMITATIONS.md docs/REFERENCES.md docs/DATA_SOURCES.md /app/docs/
COPY backend/Cargo.lock /app/backend/Cargo.lock
ENV FIELDSHIFT_ROOT=/app FIELDSHIFT_STATIC_DIR=/app/frontend/build PORT=3000
EXPOSE 3000
USER 65534:65534
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD curl --fail --silent "http://127.0.0.1:${PORT}/v1/health" || exit 1
CMD ["fieldshift-api"]
