#!/usr/bin/env bash

set -euo pipefail

# Persists local Identity secrets under .temp/dev-runtime so the API and the
# worker share them across restarts: MFA ciphertexts written by one process
# must stay readable by the other.
load_dev_identity_keys() {
  local key_dir="$ROOT_DIR/.temp/dev-runtime"
  local private_key="$key_dir/identity-token-private.pem"
  local public_key="$key_dir/identity-token-public.pem"

  require_cmd openssl
  mkdir -p "$key_dir"
  chmod 700 "$key_dir"

  if [[ -z "${NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM:-}" || -z "${NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM:-}" ]]; then
    if [[ ! -s "$private_key" || ! -s "$public_key" ]]; then
      openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
        -out "$private_key" 2>/dev/null
      openssl pkey -in "$private_key" -pubout -out "$public_key" 2>/dev/null
      chmod 600 "$private_key" "$public_key"
    fi
    NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM="$(<"$private_key")"
    NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM="$(<"$public_key")"
    export NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM
  fi

  export NVBES_IDENTITY_TOKEN_ISSUER="${NVBES_IDENTITY_TOKEN_ISSUER:-http://localhost:8080}"
  export NVBES_IDENTITY_TOKEN_KEY_ID="${NVBES_IDENTITY_TOKEN_KEY_ID:-nvbes-local-identity-key}"
  export NVBES_IDENTITY_TOKEN_AUDIENCES="${NVBES_IDENTITY_TOKEN_AUDIENCES:-nvbes-account-service}"

  NVBES_IDENTITY_MFA_ENCRYPTION_KEY="${NVBES_IDENTITY_MFA_ENCRYPTION_KEY:-$(dev_secret "$key_dir/identity-mfa.key")}"
  NVBES_IDENTITY_RATE_LIMIT_KEY="${NVBES_IDENTITY_RATE_LIMIT_KEY:-$(dev_secret "$key_dir/identity-rate-limit.key")}"
  export NVBES_IDENTITY_MFA_ENCRYPTION_KEY NVBES_IDENTITY_RATE_LIMIT_KEY
}

# Prints a base64 32-byte secret, generating it once per checkout.
dev_secret() {
  local path="$1"
  if [[ ! -s "$path" ]]; then
    openssl rand -base64 32 >"$path"
    chmod 600 "$path"
  fi
  tr -d '\n' <"$path"
}
