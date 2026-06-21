#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

sudo mkdir -p \
  /usr/local/cargo/git \
  /usr/local/cargo/registry \
  /workspaces/nvbes/node_modules \
  /workspaces/nvbes/pnpm-store \
  /workspaces/nvbes/target
sudo chown -R "$(id -u):$(id -g)" \
  /usr/local/cargo/git \
  /usr/local/cargo/registry \
  /workspaces/nvbes/node_modules \
  /workspaces/nvbes/pnpm-store \
  /workspaces/nvbes/target

if [ ! -f .env ]; then
  cp .env.example .env
  sed -i \
    -e 's#NVBES_DATABASE_URL="postgres://postgres:postgres@localhost:5432/nvbes"#NVBES_DATABASE_URL="postgres://postgres:postgres@postgres:5432/nvbes"#' \
    -e 's#NVBES_REDIS_URL="redis://localhost:6379"#NVBES_REDIS_URL="redis://:redis_dev@redis:6379"#' \
    -e 's#NVBES_API_BASE_URL="http://localhost:4000"#NVBES_API_BASE_URL="http://localhost:4000"#' \
    -e 's#NVBES_IDENTITY_API_BASE_URL="http://localhost:4000"#NVBES_IDENTITY_API_BASE_URL="http://localhost:4000"#' \
    -e 's#NVBES_DRIVE_API_BASE_URL="http://localhost:4002"#NVBES_DRIVE_API_BASE_URL="http://localhost:4002"#' \
    .env
  {
    printf '\n'
    printf 'NVBES_DRIVE_DATABASE_URL="postgres://postgres:postgres@postgres:5432/nvbes_drive"\n'
    printf 'VITE_DRIVE_API_PROXY_TARGET="http://localhost:4002"\n'
  } >> .env
fi

if ! command -v pnpm >/dev/null 2>&1; then
  npm install -g pnpm@11.1.3
fi
pnpm install
pnpm dlx lefthook install

printf '\nDev container ready. Run: pnpm db:migrate && pnpm dev\n'
