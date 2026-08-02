#!/usr/bin/env bash

# ==============================================================================
# Script d'application des migrations de bases de données - STAGING (Multi-DB)
# Usage:
#   NVBES_ALLOW_STAGING_MIGRATION=yes bash scripts/migrate-staging.sh
# ==============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
source "$SCRIPT_DIR/lib/test-env.sh"

require_cmd cargo

cd "$ROOT_DIR"

if [ "${NVBES_ALLOW_STAGING_MIGRATION:-}" != "yes" ]; then
  fail "Veuillez definir NVBES_ALLOW_STAGING_MIGRATION=yes apres avoir verifie le point de restauration Staging"
fi

log_step "Preflight des migrations Staging (cargo check)"
cargo check --workspace

log_step "Migration de la base Identity Staging (nvbes_identity)"
if [ -n "${NVBES_STAGING_IDENTITY_DATABASE_URL:-}" ]; then
  NVBES_ENV=staging \
    NVBES_DATABASE_URL="$NVBES_STAGING_IDENTITY_DATABASE_URL" \
    cargo run -p nvbes-identity-service -- --migrate
fi

log_step "Migration de la base Account Staging (nvbes_account)"
if [ -n "${NVBES_STAGING_ACCOUNT_DATABASE_URL:-}" ]; then
  NVBES_ENV=staging \
    NVBES_DATABASE_URL="$NVBES_STAGING_ACCOUNT_DATABASE_URL" \
    cargo run -p nvbes-account-service -- --migrate
fi

log_step "Migration de la base Cloud Staging (nvbes_cloud)"
if [ -n "${NVBES_STAGING_CLOUD_DATABASE_URL:-}" ]; then
  NVBES_ENV=staging \
    NVBES_DATABASE_URL="$NVBES_STAGING_CLOUD_DATABASE_URL" \
    cargo run -p nvbes-cloud-service -- --migrate
fi

log_step "Migrations Staging terminees avec succes !"
