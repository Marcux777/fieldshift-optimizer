<!-- AI-assisted (OpenBot agent) -->
# Rust hosting preparation

Status: prepared, **not deployed**. Sources checked 2026-11-14. [V] means verified in official documentation; [N] means account/runtime verification still required. No provider account, billing setting, publication or public endpoint was changed.

## Preferred: one Render Free Web Service

[V] Render supports building `backend/Dockerfile` with repository-root context. One image contains the Rust API, SvelteKit adapter-static output, config and verified NASA snapshot. No Python, report replay, database or sidecar is needed. [Docker support](https://render.com/docs/docker).

[V] Free compute is 0.1 CPU / 512 MB RAM. [Compute plans](https://render.com/docs/compute-plans).

[V] The workspace gets 750 free instance hours per month. Services sleep after 15 minutes without traffic; wake-up takes about a minute. Storage is ephemeral, and Free cannot attach persistent disks. Without a payment method, excess bandwidth suspends Free services and exhausted build allowance disables new builds. [Free service limits](https://render.com/docs/free).

[N] Actual signup eligibility/card verification for the user's account was not exercised. The docs explicitly describe operation without a payment method; that is not a guarantee about a particular signup. Keep Free selected and do not add paid resources or billing without authorization. [Free service limits](https://render.com/docs/free).

After explicit deploy authorization, configure repository `Marcux777/fieldshift-optimizer`, branch `feat/rust-svelte-migration`, Dockerfile `backend/Dockerfile`, build context `.`, Free instance, health path `/v1/health`, `FIELDSHIFT_ROOT=/app`, `FIELDSHIFT_STATIC_DIR=/app/frontend/build`, `PORT=10000` (`fieldshift-api` binds `0.0.0.0:$PORT` automatically when `PORT` is set, or uses `BIND_ADDRESS` when explicitly provided). Render's documented default port is 10000. Disable automatic deploys until independent review is complete. [Web service ports](https://render.com/docs/web-services).

[N] Shared free-tier CPU, cold starts and request timeouts need a hosted check before promising response times. Host CLI measurements are not hosted API latency. Bake the snapshot into the image; runtime writes are disposable. Do not collect new NASA data on startup.

## Other options checked

| Provider | Free allowance / limits | Card and dormancy | Assessment |
| --- | --- | --- | --- |
| Koyeb | [V] One Free preview instance; unsuitable for production use | [V] Free sleeps after one hour. Signup requires card; FAQ describes a $29 authorization hold and prorated charge for the selected plan | Not preferred for strict no-cost onboarding. [Instances](https://www.koyeb.com/docs/reference/instances), [Pricing FAQ](https://www.koyeb.com/docs/faqs/pricing), [Sleep behavior](https://www.koyeb.com/docs/run-and-scale/scale-to-zero) |
| Cloud Run | [V] Request-based free tier: 180,000 vCPU-s, 360,000 GiB-s and 2 million requests/month, aggregated per billing account; overage billed | [V] Billing account required. [N] Card/account eligibility and exact cold-start behavior not tested | Can run the container, but not a zero-billing guarantee. [Pricing](https://cloud.google.com/run/pricing), [Billing requirement](https://docs.cloud.google.com/free/docs/free-cloud-features) |
| Fly.io | [V] New-account trial is $1 credit, up to seven days; trial machines stop after five minutes | [V] Trial needs no card; paid continuation requires payment | Trial is not durable free hosting. [Trial terms](https://docs.fly.io/about/free-trial) |

## Reproduce locally

Build from the repository root:

```bash
docker build -f backend/Dockerfile -t fieldshift:rust-migration .
```

After confirming no compatible local service container already exists, create it once:

```bash
docker run --name fieldshift-rust -d --read-only --tmpfs /tmp:rw,noexec,nosuid,size=32m -p 127.0.0.1:3000:3000 fieldshift:rust-migration
```

Reuse with `docker start fieldshift-rust` after stopping it. Inspect with `docker logs fieldshift-rust`, `docker inspect fieldshift-rust`, and `curl --fail http://127.0.0.1:3000/v1/health`. It binds only to localhost in this recipe. Do not replace the Python/oracle containers or redirect their mounts.

The image runs UID 65534. Runtime contains only the compiled Rust binaries, frontend assets and required data/config/docs; compiler, Node, Python, fixtures and reports stay outside the final stage. CI checks both `/v1/health` and `/`, plus non-root/no-Python invariants. API must serve real computed data before these gates can pass.

## Offline scientific reproduction

```bash
CARGO_TARGET_DIR=/tmp/fs-target-mlops cargo run --locked --release --manifest-path backend/Cargo.toml -p fieldshift-cli -- run-evaluation --output artifacts/rust/reproduction
```

Output must be a new directory. CLI never overwrites `reports/`, protocol, config or frozen snapshots. New collections require `collect-snapshot --refresh --output <new-directory-outside-data/snapshots>`; raw-only schema 2 is explicit. Existing schema 1 snapshots are read and hashed without rewriting or interpreting Parquet. `-999` remains missing (NaN), never zero. The default collector reuses and verifies the matching existing request without a network call.
