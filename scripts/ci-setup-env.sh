#!/usr/bin/env bash

# ==============================================================================
# Script d'initialisation CLI des variables et secrets GitHub Actions (CI/CD)
# Usage:
#   bash scripts/ci-setup-env.sh [--dry-run]
# ==============================================================================

set -euo pipefail

DRY_RUN=false
if [[ "${1:-}" = "--dry-run" ]]; then
  DRY_RUN=true
  echo "==> Mode simulation (dry-run) active. Aucune modification ne sera envoyee a GitHub."
fi

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "error: commande requise manquante: $1" >&2; exit 1; }
}

require_cmd gh

echo "==> Verification de l'authentification GitHub CLI..."
gh auth status >/dev/null 2>&1 || { echo "error: gh n'est pas authentifie. Veuillez executer 'gh auth login' d'abord." >&2; exit 1; }

set_secret() {
  local name="$1"
  local value="$2"
  if [[ "$DRY_RUN" = true ]]; then
    echo "[DRY-RUN] gh secret set $name --body '***'"
  else
    echo "$value" | gh secret set "$name"
    echo "  ✓ Secret $name defini."
  fi
}

set_variable() {
  local name="$1"
  local value="$2"
  if [[ "$DRY_RUN" = true ]]; then
    echo "[DRY-RUN] gh variable set $name --body '$value'"
  else
    gh variable set "$name" --body "$value"
    echo "  ✓ Variable $name definie."
  fi
}

echo "==> Configuration des Variables d'Environnement CI/CD..."
set_variable "NVBES_ENV" "production"
set_variable "POSTHOG_CLI_HOST" "https://eu.posthog.com"
set_variable "POSTHOG_CLI_PROJECT_ID" "194777"
set_variable "GRAFANA_FARO_SOURCEMAP_ENDPOINT" "https://faro-api.grafana.net/sourcemaps"
set_variable "GRAFANA_CLOUD_STACK_ID" "123456"

echo "==> Configuration des Secrets CI/CD..."
set_secret "SCALEWAY_SECRET_KEY" "${SCALEWAY_SECRET_KEY:-scw_sec_sample_key}"
set_secret "SCALEWAY_PROJECT_ID" "${SCALEWAY_PROJECT_ID:-scw_project_sample_id}"
set_secret "CLOUDFLARE_API_TOKEN" "${CLOUDFLARE_API_TOKEN:-sample_cf_token}"
set_secret "CLOUDFLARE_ACCOUNT_ID" "${CLOUDFLARE_ACCOUNT_ID:-sample_cf_account_id}"
set_secret "SENTRY_AUTH_TOKEN" "${SENTRY_AUTH_TOKEN:-sample_sentry_token}"

echo "==> Setup termine avec succes !"
