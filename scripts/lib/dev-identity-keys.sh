#!/usr/bin/env bash

set -euo pipefail

load_dev_identity_keys() {
  local key_dir="$ROOT_DIR/.temp/dev-runtime"
  local private_key="$key_dir/identity-token-private.pem"
  local public_key="$key_dir/identity-token-public.pem"

  if [[ -z "${NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM:-}" || -z "${NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM:-}" ]]; then
    require_cmd openssl
    mkdir -p "$key_dir"
    chmod 700 "$key_dir"
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

  export NVBES_IDENTITY_TOKEN_ISSUER="${NVBES_IDENTITY_TOKEN_ISSUER:-http://127.0.0.1:3060}"
  export NVBES_IDENTITY_TOKEN_KEY_ID="${NVBES_IDENTITY_TOKEN_KEY_ID:-nvbes-local-identity-key}"
  export NVBES_IDENTITY_TOKEN_AUDIENCES="${NVBES_IDENTITY_TOKEN_AUDIENCES:-nvbes-account,nvbes-billing}"
  export NVBES_IDENTITY_PUBLIC_KEY_PEM="${NVBES_IDENTITY_PUBLIC_KEY_PEM:-$NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM}"
}
