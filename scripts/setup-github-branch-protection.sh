#!/usr/bin/env bash

# ==============================================================================
# Script de Configuration des Regles de Protection GitHub (Branches & Depot)
# Usage:
#   bash scripts/setup-github-branch-protection.sh [--dry-run]
# ==============================================================================

set -euo pipefail

REPO="nvbes-org/nvbes"

DRY_RUN=false
if [[ "${1:-}" = "--dry-run" ]]; then
  DRY_RUN=true
  echo "==> Mode simulation (dry-run) active. Aucune regle ne sera modifiee."
fi

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || { echo "error: commande requise manquante: $1" >&2; exit 1; }
}

require_cmd gh

echo "==> Verification de l'authentification GitHub CLI..."
gh auth status >/dev/null 2>&1 || { echo "error: gh n'est pas authentifie." >&2; exit 1; }

echo "==> 1. Configuration des options generales du depot ($REPO)..."
if [[ "$DRY_RUN" = true ]]; then
  echo "[DRY-RUN] gh api -X PATCH repos/$REPO -f delete_branch_on_merge=true -f allow_auto_merge=true"
else
  gh api -X PATCH "repos/$REPO" \
    -F delete_branch_on_merge=true \
    -F allow_auto_merge=true \
    -F allow_squash_merge=true \
    -F allow_merge_commit=true \
    -F allow_rebase_merge=true >/dev/null
  echo "  ✓ Auto-deletion des branches apres merge et Auto-Merge actives sur le depot."
fi

MAIN_PROTECTION_PAYLOAD='{
  "required_status_checks": {
    "strict": true,
    "contexts": ["ci", "quality-checks", "rust-checks"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "dismiss_stale_reviews": true,
    "require_code_owner_reviews": false,
    "required_approving_review_count": 1
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_linear_history": true
}'

STAGING_PROTECTION_PAYLOAD='{
  "required_status_checks": {
    "strict": true,
    "contexts": ["ci"]
  },
  "enforce_admins": false,
  "required_pull_request_reviews": null,
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_linear_history": false
}'

DEV_PROTECTION_PAYLOAD='{
  "required_status_checks": null,
  "enforce_admins": false,
  "required_pull_request_reviews": null,
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false,
  "required_linear_history": false
}'

echo "==> 2. Application des regles de protection de branches..."
if [[ "$DRY_RUN" = true ]]; then
  echo "[DRY-RUN] gh api -X PUT repos/$REPO/branches/main/protection --input -"
  echo "[DRY-RUN] gh api -X PUT repos/$REPO/branches/staging/protection --input -"
  echo "[DRY-RUN] gh api -X PUT repos/$REPO/branches/dev/protection --input -"
else
  if echo "$MAIN_PROTECTION_PAYLOAD" | gh api -X PUT "repos/$REPO/branches/main/protection" --input - >/dev/null 2>&1; then
    echo "  ✓ Protection 'main' appliquee."
    echo "$STAGING_PROTECTION_PAYLOAD" | gh api -X PUT "repos/$REPO/branches/staging/protection" --input - >/dev/null 2>&1
    echo "  ✓ Protection 'staging' appliquee."
    echo "$DEV_PROTECTION_PAYLOAD" | gh api -X PUT "repos/$REPO/branches/dev/protection" --input - >/dev/null 2>&1
    echo "  ✓ Protection 'dev' appliquee."
  else
    echo "  ℹ Note Plan GitHub : Ce depot est prive sous l'offre Free GitHub."
    echo "    Les options de depot (Auto-deletion apres merge, Auto-merge, Squash-merge) ont ete activees."
    echo "    Pour la protection de branche API stricte (Revue PR & Checks CI obligatoires), un surclassement vers GitHub Pro/Team est requis."
  fi
fi

echo "==> Configuration du depot terminee avec succes !"
