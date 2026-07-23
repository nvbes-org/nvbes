#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

sanitize_node_options() {
  local preload_path

  case "${NODE_OPTIONS:-}" in
    *ms-vscode.js-debug/bootloader.js*)
      preload_path="$(printf '%s\n' "$NODE_OPTIONS" | sed -n 's/.*--require[= ]\([^ ]*ms-vscode.js-debug\/bootloader\.js\).*/\1/p')"
      if [ -z "$preload_path" ] || [ ! -e "$preload_path" ]; then
        unset NODE_OPTIONS
      fi
      ;;
  esac
}

install_node_options_guard() {
  local bashrc="$HOME/.bashrc"
  local marker="# nvbes devcontainer node options guard"

  touch "$bashrc"
  if grep -Fq "$marker" "$bashrc"; then
    return
  fi

  cat >> "$bashrc" <<'EOF'

# nvbes devcontainer node options guard
if [[ "${NODE_OPTIONS:-}" == *"ms-vscode.js-debug/bootloader.js"* ]]; then
  __nvbes_node_preload="$(printf '%s\n' "$NODE_OPTIONS" | sed -n 's/.*--require[= ]\([^ ]*ms-vscode.js-debug\/bootloader\.js\).*/\1/p')"
  if [ -z "$__nvbes_node_preload" ] || [ ! -e "$__nvbes_node_preload" ]; then
    unset NODE_OPTIONS
  fi
  unset __nvbes_node_preload
fi
EOF
}

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

sanitize_node_options
install_node_options_guard

sudo mkdir -p \
  "$HOME/.cache" \
  "$HOME/.codex" \
  "$HOME/.config" \
  "$HOME/.gnupg" \
  "$HOME/.local/share" \
  "$HOME/.vscode-server" \
  "$HOME/.vscode-server-insiders" \
  /usr/local/cargo/git \
  /usr/local/cargo/registry \
  /workspaces/nvbes/node_modules \
  /workspaces/nvbes/pnpm-store \
  /workspaces/nvbes/target
sudo chown -R "$(id -u):$(id -g)" \
  "$HOME/.cache" \
  "$HOME/.codex" \
  "$HOME/.config" \
  "$HOME/.gnupg" \
  "$HOME/.local/share" \
  "$HOME/.vscode-server" \
  "$HOME/.vscode-server-insiders" \
  /usr/local/cargo/git \
  /usr/local/cargo/registry \
  /workspaces/nvbes/node_modules \
  /workspaces/nvbes/pnpm-store \
  /workspaces/nvbes/target
chmod 700 "$HOME/.gnupg" "$HOME/.ssh"

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
pnpm install --frozen-lockfile --prefer-offline
pnpm dlx lefthook install

printf '\nDev container ready. Run: pnpm db:migrate && pnpm dev\n'
