#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

configure_git() {
  local current_name current_email github_login github_name github_email signing_key

  mkdir -p "$HOME/.config/gh" "$HOME/.config/git" "$HOME/.ssh"
  chmod 700 "$HOME/.ssh"

  if ! git config --global --get-all safe.directory | grep -Fxq "$ROOT_DIR"; then
    git config --global --add safe.directory "$ROOT_DIR"
  fi

  if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    gh auth setup-git >/dev/null
  else
    printf 'GitHub CLI is not authenticated yet. Run: gh auth login\n'
  fi

  current_name="$(git config --global --get user.name || true)"
  if [ -z "$current_name" ] && command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    github_name="$(gh api user --jq '.name // empty' 2>/dev/null || true)"
    github_login="$(gh api user --jq '.login // empty' 2>/dev/null || true)"
    git config --global user.name "${github_name:-$github_login}"
  fi

  current_email="$(git config --global --get user.email || true)"
  if [ -z "$current_email" ] && command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
    github_email="$(gh api user/emails --jq 'map(select(.primary == true))[0].email // empty' 2>/dev/null || true)"
    github_login="$(gh api user --jq '.login // empty' 2>/dev/null || true)"
    git config --global user.email "${github_email:-$github_login@users.noreply.github.com}"
  fi

  signing_key="${NVBES_GIT_SIGNING_KEY:-}"
  if [ -z "$signing_key" ] && [ -f "$HOME/.ssh/id_ed25519.pub" ]; then
    signing_key="$HOME/.ssh/id_ed25519.pub"
  elif [ -z "$signing_key" ] && [ -f "$HOME/.ssh/id_rsa.pub" ]; then
    signing_key="$HOME/.ssh/id_rsa.pub"
  fi

  if [ -n "$signing_key" ]; then
    git config --global gpg.format ssh
    git config --global user.signingkey "$signing_key"
    git config --global commit.gpgsign true
  fi
}

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

configure_git

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
