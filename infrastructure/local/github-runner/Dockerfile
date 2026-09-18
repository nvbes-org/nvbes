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
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg -o /etc/apt/keyrings/docker.asc && \
    chmod a+r /etc/apt/keyrings/docker.asc && \
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "$VERSION_CODENAME") stable" > /etc/apt/sources.list.d/docker.list && \
    apt-get update && apt-get install -y --no-install-recommends docker-ce-cli docker-buildx-plugin docker-compose-plugin && \
    rm -rf /var/lib/apt/lists/*

# Install Node.js 24 and pnpm / corepack
RUN curl -fsSL https://deb.nodesource.com/setup_24.x | bash - && \
    apt-get install -y --no-install-recommends nodejs && \
    npm install -g pnpm@11.18.0 corepack@latest && \
    corepack enable && \
    rm -rf /var/lib/apt/lists/*

# Install Rust toolchain
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile default && \
    /usr/local/cargo/bin/rustup component add rustfmt clippy && \
    /usr/local/cargo/bin/rustup toolchain install 1.98.1 --component rustfmt,clippy && \
    chmod -R 777 /usr/local/rustup /usr/local/cargo

# Install Terraform
RUN curl -fsSL https://apt.releases.hashicorp.com/gpg | gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg && \
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
    curl -o actions-runner-linux.tar.gz -L https://github.com/actions/runner/releases/download/v${RUNNER_VERSION}/actions-runner-linux-${RUNNER_ARCH}-${RUNNER_VERSION}.tar.gz && \
    tar xzf ./actions-runner-linux.tar.gz && \
    rm actions-runner-linux.tar.gz && \
    ./bin/installdependencies.sh && \
    chown -R runner:runner /home/runner

COPY --chown=runner:runner entrypoint.sh /home/runner/entrypoint.sh
RUN chmod +x /home/runner/entrypoint.sh

USER runner
ENTRYPOINT ["/home/runner/entrypoint.sh"]
