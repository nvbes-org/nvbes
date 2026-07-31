#!/usr/bin/env bash

# ==============================================================================
# Script de verification Preflight & Initialisation de l'environnement Staging
# Usage:
#   bash scripts/setup-staging.sh [--dry-run]
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

DRY_RUN=false
if [ "${1:-}" = "--dry-run" ]; then
  DRY_RUN=true
  log_step "Mode simulation Staging (dry-run) active"
fi

log_step "Verification des outils requis pour Staging"
require_cmd cargo
require_cmd pnpm
require_cmd node
require_cmd gh

log_step "Verification de l'existence de la configuration Staging"
if [ ! -f "$ROOT_DIR/.env.staging" ]; then
  printf 'Info: .env.staging absent. Copie depuis .env.staging.example...\n'
  if [ "$DRY_RUN" = false ]; then
    cp "$ROOT_DIR/.env.staging.example" "$ROOT_DIR/.env.staging"
  fi
fi

log_step "Preflight Staging valide avec succes !"
