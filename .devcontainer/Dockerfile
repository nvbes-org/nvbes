FROM mcr.microsoft.com/devcontainers/base:ubuntu-24.04

ARG NODE_VERSION=25.8.0
ARG PNPM_VERSION=11.1.3
ARG USERNAME=vscode

ENV DEBIAN_FRONTEND=noninteractive
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH=/usr/local/cargo/bin:/usr/local/node/bin:${PATH}

SHELL ["/bin/bash", "-o", "pipefail", "-c"]

RUN apt-get update \
  && apt-get install -y --no-install-recommends \
    build-essential \
    ca-certificates \
    clang \
    cmake \
    curl \
    gh \
    git \
    gnupg \
    golang-go \
    jq \
    lld \
    libclang-dev \
    libxml2-dev \
    libpq-dev \
    libssl-dev \
    libxmlsec1-dev \
    libxmlsec1-openssl \
    openssh-client \
    pkg-config \
    postgresql-client \
    protobuf-compiler \
    python3 \
    python3-pip \
    python3-venv \
    redis-tools \
    ripgrep \
    shellcheck \
    unzip \
    wget \
    zstd \
  && rm -rf /var/lib/apt/lists/*

RUN arch="$(dpkg --print-architecture)" \
  && case "${arch}" in \
    amd64) node_arch="x64" ;; \
    arm64) node_arch="arm64" ;; \
    *) echo "Unsupported architecture: ${arch}" >&2; exit 1 ;; \
  esac \
  && curl -fsSL "https://nodejs.org/dist/v${NODE_VERSION}/node-v${NODE_VERSION}-linux-${node_arch}.tar.xz" -o /tmp/node.tar.xz \
  && mkdir -p /usr/local/node \
  && tar -xJf /tmp/node.tar.xz -C /usr/local/node --strip-components=1 \
  && rm /tmp/node.tar.xz \
  && npm install -g "pnpm@${PNPM_VERSION}"

RUN curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs \
  | sh -s -- -y --profile default --default-toolchain stable --no-modify-path \
  && rustup component add rustfmt clippy \
  && chmod -R a+w "${RUSTUP_HOME}" "${CARGO_HOME}"

RUN cargo install sqlx-cli --locked --no-default-features --features postgres

RUN curl --proto '=https' --tlsv1.2 -fsSL https://get.opentofu.org/install-opentofu.sh -o /tmp/install-opentofu.sh \
  && chmod +x /tmp/install-opentofu.sh \
  && /tmp/install-opentofu.sh --install-method deb \
  && rm /tmp/install-opentofu.sh

RUN mkdir -p /workspace-cache/pnpm /workspace-cache/cargo-target \
  && chown -R "${USERNAME}:${USERNAME}" /workspace-cache

USER ${USERNAME}

RUN mkdir -p \
    /home/${USERNAME}/.cache \
    /home/${USERNAME}/.config/gh \
    /home/${USERNAME}/.config/git \
    /home/${USERNAME}/.local/bin \
    /home/${USERNAME}/.local/share \
    /home/${USERNAME}/.ssh \
    /home/${USERNAME}/.vscode-server \
    /home/${USERNAME}/.vscode-server-insiders \
  && chown -R "${USERNAME}:${USERNAME}" \
    /home/${USERNAME}/.cache \
    /home/${USERNAME}/.config \
    /home/${USERNAME}/.local \
    /home/${USERNAME}/.ssh \
    /home/${USERNAME}/.vscode-server \
    /home/${USERNAME}/.vscode-server-insiders \
  && chmod 700 /home/${USERNAME}/.ssh

WORKDIR /workspaces/nvbes
