#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

runtime_dir="$(mktemp -d)"
trap 'rm -rf "$runtime_dir"' EXIT
runtime_openapi="$runtime_dir/identity-service.openapi.json"

# This target is intentionally infrastructure-free. Database-backed behavior is
# owned by identity-service:test and identity-service:test-security; the contract
# gate only proves that the runtime document, committed schemas and generated
# TypeScript stay identical.
cargo run --quiet --package nvbes-identity-service --locked -- --export-openapi > "$runtime_openapi"
IDENTITY_RUNTIME_OPENAPI="$runtime_openapi" \
  node --test libs/ts/identity-sdk-core/openapi.contract.test.mjs
