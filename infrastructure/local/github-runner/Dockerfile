FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    jq \
    git \
    tar \
    unzip \
    build-essential \
    sudo \
    ca-certificates \
    gnupg \
    lsb-release \
    perl \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install Docker CLI
RUN install -m 0755 -d /etc/apt/keyrings && \
    curl --proto '=https' --tlsv1.2 -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc && \
    chmod a+r /etc/apt/keyrings/docker.asc && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "$VERSION_CODENAME") stable" > /etc/apt/sources.list.d/docker.list && \
    apt-get update && apt-get install -y --no-install-recommends docker-ce-cli docker-buildx-plugin docker-compose-plugin && \
    rm -rf /var/lib/apt/lists/*

# Install Node.js 24 and pnpm / corepack
RUN mkdir -p /etc/apt/keyrings && \
    curl --proto '=https' --tlsv1.2 -fsSL https://deb.nodesource.com/gpgkey/nodesource-repo.gpg.key | gpg --dearmor -o /etc/apt/keyrings/nodesource.gpg && \
    echo "deb [signed-by=/etc/apt/keyrings/nodesource.gpg] https://deb.nodesource.com/node_24.x nodistro main" > /etc/apt/sources.list.d/nodesource.list && \
    apt-get update && apt-get install -y --no-install-recommends nodejs && \
    npm install -g --ignore-scripts pnpm@11.18.0 corepack@0.31.0 && \
    corepack enable && \
    rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://static.rust-lang.org/rustup/dist/$(uname -m)-unknown-linux-gnu/rustup-init -o /tmp/rustup-init && \
    chmod 0755 /tmp/rustup-init && \
    /tmp/rustup-init -y --default-toolchain stable --profile default && \
    rm -f /tmp/rustup-init && \
    /usr/local/cargo/bin/rustup component add rustfmt clippy && \
    /usr/local/cargo/bin/rustup toolchain install 1.98.1 --component rustfmt,clippy && \
    chmod -R 0775 /usr/local/rustup /usr/local/cargo

# Install Terraform
RUN curl --proto '=https' --tlsv1.2 -fsSL https://apt.releases.hashicorp.com/gpg | gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg && \
    echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main" > /etc/apt/sources.list.d/hashicorp.list && \
    apt-get update && apt-get install -y --no-install-recommends terraform && \
    rm -rf /var/lib/apt/lists/*

# Create runner user with passwordless sudo
RUN useradd -m -s /bin/bash runner && \
    usermod -aG sudo runner && \
    echo "runner ALL=(ALL) NOPASSWD:ALL" >> /etc/sudoers

WORKDIR /home/runner

# Download runner
ARG RUNNER_VERSION=2.336.0
RUN ARCH=$(dpkg --print-architecture) && \
    if [ "$ARCH" = "amd64" ]; then RUNNER_ARCH="x64"; elif [ "$ARCH" = "arm64" ]; then RUNNER_ARCH="arm64"; fi && \
    curl --proto '=https' --tlsv1.2 -fsSL -o actions-runner-linux.tar.gz https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-${RUNNER_ARCH}-${RUNNER_VERSION}.tar.gz && \
    tar xzf ./actions-runner-linux.tar.gz && \
    rm actions-runner-linux.tar.gz && \
    ./bin/installdependencies.sh && \
    chown -R runner:runner /home/runner

COPY --chmod=0555 entrypoint.sh /home/runner/entrypoint.sh

USER runner
ENTRYPOINT ["/home/runner/entrypoint.sh"]
