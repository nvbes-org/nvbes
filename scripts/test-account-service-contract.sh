#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

runtime_dir="$(mktemp -d)"
trap 'rm -rf "$runtime_dir"' EXIT
runtime_openapi="$runtime_dir/account-service.openapi.json"

# This target is intentionally infrastructure-free. Database-backed behavior is
# owned by account-service:test and account-service:test-security; the contract
# gate only proves that the runtime document, committed schemas and generated
# TypeScript stay identical.
cargo run --quiet --package nvbes-account-service --locked -- --export-openapi > "$runtime_openapi"
ACCOUNT_RUNTIME_OPENAPI="$runtime_openapi" \
  node --test libs/ts/identity-sdk-core/openapi.contract.test.mjs
