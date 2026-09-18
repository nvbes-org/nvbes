FROM rust:1.98.1-slim-bookworm@sha256:ebd900bae66fd508b466cef82d64a83a5fb34682e4c8b2797a42908bddc95a57 AS builder

ARG CARGO_PACKAGE
ARG CARGO_BINARY
ARG APPLICATION_PATH
ARG SOURCE_DATE_EPOCH=0
ARG DEBIAN_SNAPSHOT=20260918T000000Z
ENV LC_ALL=C \
    TZ=UTC \
    SOURCE_DATE_EPOCH="${SOURCE_DATE_EPOCH}" \
    CARGO_INCREMENTAL=0 \
    RUSTFLAGS="--remap-path-prefix=/usr/src/nvbes=/src"

WORKDIR /usr/src/nvbes

RUN test -n "${CARGO_PACKAGE}" \
    && test -n "${CARGO_BINARY}" \
    && test -n "${APPLICATION_PATH}" \
    && rm -f /etc/apt/sources.list.d/debian.sources \
    && printf '%s\n' \
      "deb http://snapshot.debian.org/archive/debian/${DEBIAN_SNAPSHOT}/ bookworm main" \
      "deb http://snapshot.debian.org/archive/debian-security/${DEBIAN_SNAPSHOT}/ bookworm-security main" \
      > /etc/apt/sources.list \
    && apt-get -o Acquire::Check-Valid-Until=false update \
    && apt-get install -y --no-install-recommends \
      libclang-dev \
      libssl-dev \
      libxml2-dev \
      libxmlsec1-dev \
      libxmlsec1-openssl \
      make \
      perl \
      pkg-config \
      protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY apps ./apps
COPY contracts ./contracts
COPY libs ./libs
COPY vendor ./vendor

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/nvbes/target \
    cargo build --locked --release --package "${CARGO_PACKAGE}" --bin "${CARGO_BINARY}" \
    && install -D "target/release/${CARGO_BINARY}" /out/nvbes-app \
    && if [ -d "${APPLICATION_PATH}/migrations" ]; then \
      find "${APPLICATION_PATH}/migrations" -type f -exec sh -c \
        'for file do install -D "$file" "/out/$file"; done' sh {} +; \
    fi

FROM debian:bookworm-slim@sha256:7b140f374b289a7c2befc338f42ebe6441b7ea838a042bbd5acbfca6ec875818 AS runtime

ARG DEBIAN_SNAPSHOT=20260918T000000Z
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
COPY --from=builder --chown=10001:10001 /out/ /app/

USER 10001:10001
STOPSIGNAL SIGTERM
ENTRYPOINT ["/app/nvbes-app"]
