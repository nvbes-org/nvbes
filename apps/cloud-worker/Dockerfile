# Multi-stage Dockerfile for cloud-worker (Scaleway Serverless compatible)
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
COPY apps/cloud-worker ./apps/cloud-worker
COPY apps/cloud-service ./apps/cloud-service
COPY libs ./libs

# Build release binary for cloud-worker
RUN cargo build --release --bin nvbes-cloud-worker

# Final minimal runtime image
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/nvbes/target/release/nvbes-cloud-worker /app/cloud-worker
COPY --from=builder /usr/src/nvbes/apps/cloud-service/migrations /app/cloud-service/migrations

ENV NVBES_CLOUD_WORKER_METRICS_BIND_ADDR="0.0.0.0:4101"

ENTRYPOINT ["/app/cloud-worker"]
CMD ["run-once"]
