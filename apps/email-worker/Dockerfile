FROM rust:1.98.1-slim-bookworm@sha256:ebd900bae66fd508b466cef82d64a83a5fb34682e4c8b2797a42908bddc95a57 AS builder

ARG TARGETPLATFORM
ARG SOURCE_DATE_EPOCH=0
ARG DEBIAN_SNAPSHOT=20260713T000000Z
ENV LC_ALL=C \
    TZ=UTC \
    SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH}" \
    CARGO_INCREMENTAL=0 \
    RUSTFLAGS="--remap-path-prefix=/usr/src/nvbes=/src"

WORKDIR /usr/src/nvbes

RUN rm -f /etc/apt/sources.list.d/debian.sources \
    && printf '%s\n' \
      "deb http://snapshot.debian.org/archive/debian/${DEBIAN_SNAPSHOT}/ bookworm main" \
      "deb http://snapshot.debian.org/archive/debian-security/${DEBIAN_SNAPSHOT}/ bookworm-security main" \
      > /etc/apt/sources.list \
    && apt-get -o Acquire::Check-Valid-Until=false update \
    && apt-get install -y --no-install-recommends pkg-config libssl-dev perl make \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY contracts ./contracts
COPY libs ./libs
COPY vendor ./vendor

RUN --mount=type=cache,id=nvbes-cargo-registry-rust-1.98.1-${TARGETPLATFORM},target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,id=nvbes-cargo-git-rust-1.98.1-${TARGETPLATFORM},target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,id=nvbes-email-worker-target-rust-1.98.1-${TARGETPLATFORM},target=/usr/src/nvbes/target,sharing=locked \
    cargo build --locked --release --bin nvbes-email-worker \
    && install -D target/release/nvbes-email-worker /out/email-worker

FROM debian:bookworm-slim@sha256:88200866dfff7ea7f5cbcb6ec7c8a701889efe6fe859fe64d6990e4b07ea4171 AS runtime

ARG DEBIAN_SNAPSHOT=20260713T000000Z
RUN rm -f /etc/apt/sources.list.d/debian.sources \
    && printf '%s\n' \
      "deb http://snapshot.debian.org/archive/debian/${DEBIAN_SNAPSHOT}/ bookworm main" \
      "deb http://snapshot.debian.org/archive/debian-security/${DEBIAN_SNAPSHOT}/ bookworm-security main" \
      > /etc/apt/sources.list \
    && apt-get -o Acquire::Check-Valid-Until=false update \
    && apt-get upgrade -y \
    && apt-get install -y --no-install-recommends ca-certificates curl libssl3 \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 nvbes \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin nvbes

WORKDIR /app
COPY --from=builder --chown=10001:10001 /out/email-worker /app/email-worker

ENV PORT="8080"

USER 10001:10001
EXPOSE 8080
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD ["curl", "--fail", "--silent", "--show-error", "http://127.0.0.1:8080/health/live"]
ENTRYPOINT ["/app/email-worker"]
