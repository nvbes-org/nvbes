#!/usr/bin/env bash
# Ensures the isolated security-test PostgreSQL is reachable, then execs the
# given command with NVBES_SECURITY_TEST_DATABASE_URL and DATABASE_URL exported.
#
# Rust Postgres tests hard-require the security URL (see libs/rust/test-utils/src/postgres.rs:
# "PostgreSQL tests cannot be skipped"). Coverage/condition/V1 llvm-cov also need
# DATABASE_URL as the mother database for `#[sqlx::test]`. CI provisions both via
# tools/ci/test-security-database.mjs; this wrapper provides the local equivalent
# so `pnpm test` works on a dev machine. No-op when a reachable security URL is
# already set (DATABASE_URL is still filled if missing).
set -euo pipefail

CONTAINER="nvbes-security-test-db"
DB="nvbes_security_test"
COVERAGE_DB="nvbes_coverage_test"
PORT="15432"

url_is_reachable() {
  local url="$1"
  if command -v pg_isready >/dev/null 2>&1; then
    # Accept any directly reachable Postgres URL (local install, CI service, etc.).
    if pg_isready -d "$url" >/dev/null 2>&1; then
      return 0
    fi
  fi
  if command -v docker >/dev/null 2>&1 && docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

url_ends_with_test_db() {
  local url="$1"
  local path
  path="$(printf '%s' "$url" | sed -E 's#^[a-z]+://[^/]+/##' | sed -E 's#[?#].*$##')"
  [[ "$path" == *_test ]]
}

ensure_coverage_database_url() {
  local security_url="$1"
  if [[ -n "${DATABASE_URL:-}" ]] && url_is_reachable "$DATABASE_URL" && url_ends_with_test_db "$DATABASE_URL"; then
    return 0
  fi
  local base
  base="$(printf '%s' "$security_url" | sed -E 's#/[^/]+/?$##')"
  local coverage_url="${base}/${COVERAGE_DB}"
  if command -v docker >/dev/null 2>&1 && docker inspect "$CONTAINER" >/dev/null 2>&1; then
    docker exec "$CONTAINER" psql -U postgres -h 127.0.0.1 -tc \
      "SELECT 1 FROM pg_database WHERE datname='${COVERAGE_DB}'" | grep -q 1 \
      || docker exec "$CONTAINER" psql -U postgres -h 127.0.0.1 -c "CREATE DATABASE ${COVERAGE_DB}" >/dev/null
  fi
  export DATABASE_URL="$coverage_url"
}

if [[ -n "${NVBES_SECURITY_TEST_DATABASE_URL:-}" ]]; then
  if url_is_reachable "$NVBES_SECURITY_TEST_DATABASE_URL" \
    && url_ends_with_test_db "$NVBES_SECURITY_TEST_DATABASE_URL"; then
    ensure_coverage_database_url "$NVBES_SECURITY_TEST_DATABASE_URL"
    exec "$@"
  fi
  if ! url_ends_with_test_db "${NVBES_SECURITY_TEST_DATABASE_URL}"; then
    echo "with-security-test-db: preset URL database must end in _test, reprovisioning locally" >&2
  else
    echo "with-security-test-db: preset URL unreachable, reprovisioning locally" >&2
  fi
  unset NVBES_SECURITY_TEST_DATABASE_URL
fi

if ! command -v docker >/dev/null 2>&1; then
  echo "with-security-test-db: docker is required (or export a reachable NVBES_SECURITY_TEST_DATABASE_URL)" >&2
  exit 1
fi

if ! docker inspect "$CONTAINER" >/dev/null 2>&1; then
  docker run -d --name "$CONTAINER" \
    -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres \
    -p "127.0.0.1:${PORT}:5432" postgres:17 >/dev/null
fi
if [[ "$(docker inspect -f '{{.State.Running}}' "$CONTAINER")" != "true" ]]; then
  docker start "$CONTAINER" >/dev/null
fi

for _ in $(seq 1 30); do
  if docker exec "$CONTAINER" pg_isready -U postgres >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
docker exec "$CONTAINER" pg_isready -U postgres >/dev/null

docker exec "$CONTAINER" psql -U postgres -h 127.0.0.1 -tc \
  "SELECT 1 FROM pg_database WHERE datname='${DB}'" | grep -q 1 \
  || docker exec "$CONTAINER" psql -U postgres -h 127.0.0.1 -c "CREATE DATABASE ${DB}" >/dev/null

docker exec "$CONTAINER" psql -U postgres -h 127.0.0.1 -tc \
  "SELECT 1 FROM pg_database WHERE datname='${COVERAGE_DB}'" | grep -q 1 \
  || docker exec "$CONTAINER" psql -U postgres -h 127.0.0.1 -c "CREATE DATABASE ${COVERAGE_DB}" >/dev/null

export NVBES_SECURITY_TEST_DATABASE_URL="postgres://postgres:postgres@127.0.0.1:${PORT}/${DB}"
export DATABASE_URL="postgres://postgres:postgres@127.0.0.1:${PORT}/${COVERAGE_DB}"
exec "$@"
