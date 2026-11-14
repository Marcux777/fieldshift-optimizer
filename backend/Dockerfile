# Run from repository root: docker build -f backend/Dockerfile .
FROM rust:1-bookworm AS build
WORKDIR /workspace
COPY backend/Cargo.toml backend/Cargo.toml
COPY backend/src backend/src
RUN cargo build --release --manifest-path backend/Cargo.toml

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=build /workspace/backend/target/release/fieldshift-api /usr/local/bin/fieldshift-api
COPY reports/20261114T181122Z /app/reports/20261114T181122Z
ENV FIELDSHIFT_ROOT=/app BIND_ADDRESS=0.0.0.0:3000
EXPOSE 3000
USER 65534:65534
CMD ["fieldshift-api"]
