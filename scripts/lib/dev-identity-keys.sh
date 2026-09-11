#!/usr/bin/env bash

set -euo pipefail

load_dev_identity_keys() {
  local key_dir="$ROOT_DIR/.temp/dev-runtime"
  local private_key="$key_dir/identity-token-private.pem"
  local public_key="$key_dir/identity-token-public.pem"

  if [ -z "${NVBES_IDENTITY_TOKEN_PRIVATE_KEY_PEM:-}" ] \
    || [ -z "${NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM:-}" ]; then
    require_cmd openssl
    mkdir -p "$key_dir"
    chmod 700 "$key_dir"
    if [ ! -s "$private_key" ] || [ ! -s "$public_key" ]; then
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
  export NVBES_IDENTITY_TOKEN_AUDIENCES="${NVBES_IDENTITY_TOKEN_AUDIENCES:-nvbes-account-service,nvbes-billing-service}"
  export NVBES_IDENTITY_PUBLIC_KEY_PEM="${NVBES_IDENTITY_PUBLIC_KEY_PEM:-$NVBES_IDENTITY_TOKEN_PUBLIC_KEY_PEM}"

  # Keep the Account authority credential separate from Identity introspection.
  # An explicitly configured remote authority must provide its own credential.
  if [ -z "${NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET:-}" ] && [ -z "${NVBES_BILLING_ACCOUNT_ORIGIN:-}" ]; then
    node "$(dirname "${BASH_SOURCE[0]}")/dev-identity-resources.mjs" "$key_dir"
    NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET="$(<"$key_dir/identity-billing-authorization-resource.secret")"
    export NVBES_ACCOUNT_BILLING_AUTHORIZATION_SECRET
  fi
  export NVBES_BILLING_ACCOUNT_ORIGIN="${NVBES_BILLING_ACCOUNT_ORIGIN:-http://127.0.0.1:3070}"

  if [ -n "${NVBES_IDENTITY_RESOURCE_SERVERS_JSON:-}" ]; then
    # Custom registries and credentials are owned by the caller; do not replace them.
    return 0
  fi
  if [ -z "${NVBES_BILLING_IDENTITY_RESOURCE_SECRET:-}" ] || [ -z "${NVBES_ACCOUNT_IDENTITY_RESOURCE_SECRET:-}" ]; then
    node "$(dirname "${BASH_SOURCE[0]}")/dev-identity-resources.mjs" "$key_dir"
  fi
  if [ -z "${NVBES_BILLING_IDENTITY_RESOURCE_SECRET:-}" ]; then
    NVBES_BILLING_IDENTITY_RESOURCE_SECRET="$(<"$key_dir/identity-billing-resource.secret")"
  fi
  if [ -z "${NVBES_ACCOUNT_IDENTITY_RESOURCE_SECRET:-}" ]; then
    NVBES_ACCOUNT_IDENTITY_RESOURCE_SECRET="$(<"$key_dir/identity-account-resource.secret")"
  fi
  export NVBES_ACCOUNT_IDENTITY_RESOURCE_SECRET
  export NVBES_ACCOUNT_IDENTITY_RESOURCE_CLIENT_ID="${NVBES_ACCOUNT_IDENTITY_RESOURCE_CLIENT_ID:-account-api-local}"
  export NVBES_BILLING_IDENTITY_RESOURCE_SECRET
  export NVBES_BILLING_IDENTITY_RESOURCE_CLIENT_ID="${NVBES_BILLING_IDENTITY_RESOURCE_CLIENT_ID:-billing-api-local}"
  if [ -z "${NVBES_IDENTITY_RESOURCE_SERVERS_JSON:-}" ]; then
    NVBES_IDENTITY_RESOURCE_SERVERS_JSON="$(node -e 'process.stdout.write(JSON.stringify([{client_id:process.env.NVBES_BILLING_IDENTITY_RESOURCE_CLIENT_ID,audience:"nvbes-billing-service",secret:process.env.NVBES_BILLING_IDENTITY_RESOURCE_SECRET},{client_id:process.env.NVBES_ACCOUNT_IDENTITY_RESOURCE_CLIENT_ID,audience:"nvbes-account-service",secret:process.env.NVBES_ACCOUNT_IDENTITY_RESOURCE_SECRET}]))')"
  fi
  export NVBES_IDENTITY_RESOURCE_SERVERS_JSON
}
