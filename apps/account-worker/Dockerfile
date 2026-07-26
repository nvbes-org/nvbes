# Multi-stage Dockerfile for account-worker (Scaleway Serverless compatible)
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /usr/src/nvbes

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifests and source files
COPY Cargo.toml Cargo.lock ./
COPY apps/account-worker ./apps/account-worker
COPY apps/account-service ./apps/account-service
COPY libs ./libs

# Build release binary for account-worker
RUN cargo build --release --bin account-worker

# Final minimal runtime image
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/nvbes/target/release/account-worker /app/account-worker
COPY --from=builder /usr/src/nvbes/apps/account-service/migrations /app/account-service/migrations

ENV NVBES_ACCOUNT_WORKER_METRICS_BIND_ADDR="0.0.0.0:4102"

ENTRYPOINT ["/app/account-worker"]
CMD ["run-housekeeping"]
