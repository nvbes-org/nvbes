#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"
case "${1:-}" in
  staging) ;;
  production)
    if [ "${RELEASE_APPROVED:-}" != production ]; then
      printf 'production gate requires RELEASE_APPROVED=production\n' >&2
      exit 1
    fi
    ;;
  *) printf 'usage: scripts/release-gate.sh <staging|production>\n' >&2; exit 1 ;;
esac

# Candidate readiness only. Deployment and opening public access are separate.
# The same verifier produces the V1 summary and the release verdict.
pnpm exec nx run test-summary:release
