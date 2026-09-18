FROM rust:1.98.1-slim-bookworm@sha256:ebd900bae66fd508b466cef82d64a83a5fb34682e4c8b2797a42908bddc95a57 AS builder

ARG TARGETPLATFORM
ARG SOURCE_DATE_EPOCH=0
ARG DEBIAN_FRONTEND=noninteractive
ENV LC_ALL=C \
    TZ=UTC \
    SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH}" \
    CARGO_INCREMENTAL=0 \
    CARGO_BUILD_JOBS=1 \
    CMAKE_BUILD_PARALLEL_LEVEL=1 \
    RUSTFLAGS="--remap-path-prefix=/usr/src/nvbes=/src"

WORKDIR /usr/src/nvbes

RUN apt-get update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev perl make \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY contracts ./contracts
COPY libs ./libs
COPY vendor ./vendor

RUN --mount=type=cache,id=nvbes-cargo-registry-rust-1.98.1-${TARGETPLATFORM},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=nvbes-cargo-git-rust-1.98.1-${TARGETPLATFORM},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=nvbes-trust-risk-service-target-rust-1.98.1-${TARGETPLATFORM},target=/usr/src/nvbes/target,sharing=locked \
    cargo build --locked --release --bin nvbes-trust-risk-service \
    && install -D target/release/nvbes-trust-risk-service /out/trust-risk-service

FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818 AS runtime

ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install -y --no-install-recommends ca-certificates curl libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 nvbes \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin nvbes

WORKDIR /app
COPY --from=builder --chown=10001:10001 /out/trust-risk-service /app/trust-risk-service

ENV NVBES_TRUST_RISK_BIND_ADDR="0.0.0.0:8080"

USER 10001:10001
EXPOSE 8080
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD ["curl", "--fail", "--silent", "--show-error", "http://127.0.0.1:8080/health/live"]
ENTRYPOINT ["/app/trust-risk-service"]
