# Multi-stage Dockerfile for billing-worker (Scaleway Serverless compatible)
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
COPY apps/billing-worker ./apps/billing-worker
COPY libs ./libs

# Build release binary for billing-worker
RUN cargo build --release --bin nvbes-billing-worker

# Final minimal runtime image
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /usr/src/nvbes/target/release/nvbes-billing-worker /app/billing-worker

ENV NVBES_BILLING_WORKER_METRICS_BIND_ADDR="0.0.0.0:4104"

ENTRYPOINT ["/app/billing-worker"]
CMD ["run-billing-jobs"]
