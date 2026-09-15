# Reproducible multi-stage image for the Identity HTTP and gRPC service.
FROM rust:1.98.1-slim-bookworm@sha256:ebd900bae66fd508b466cef82d64a83a5fb34682e4c8b2797a42908bddc95a57 AS builder

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
    && apt-get install -y --no-install-recommends \
      libssl-dev \
      libclang-dev \
      libxml2-dev \
      libxmlsec1-dev \
      libxmlsec1-openssl \
      make \
      perl \
      pkg-config \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY contracts ./contracts
COPY libs ./libs
COPY vendor ./vendor

RUN cargo build --locked --release --package nvbes-identity-service --bin nvbes-identity-service

FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818 AS runtime

ARG DEBIAN_SNAPSHOT=20260713T000000Z
RUN rm -f /etc/apt/sources.list.d/debian.sources \
    && printf '%s\n' \
      "deb http://snapshot.debian.org/archive/debian/${DEBIAN_SNAPSHOT}/ bookworm main" \
      "deb http://snapshot.debian.org/archive/debian-security/${DEBIAN_SNAPSHOT}/ bookworm-security main" \
      > /etc/apt/sources.list \
    && apt-get -o Acquire::Check-Valid-Until=false update \
    && apt-get install -y --no-install-recommends \
      ca-certificates \
      curl \
      libssl3 \
      libxmlsec1-openssl \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 nvbes \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin nvbes

WORKDIR /app

COPY --from=builder --chown=10001:10001 /usr/src/nvbes/target/release/nvbes-identity-service /app/nvbes-identity-service

ENV NVBES_API_PORT="4000" \
    NVBES_IDENTITY_GRPC_PORT="4010"

EXPOSE 4000 4010
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD ["curl", "--fail", "--silent", "--show-error", "http://127.0.0.1:4000/health"]

USER 10001:10001

ENTRYPOINT ["/app/nvbes-identity-service"]
