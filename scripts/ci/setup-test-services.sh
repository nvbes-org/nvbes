#!/usr/bin/env bash
set -euo pipefail

# Determine Operating System
OS="$(uname -s)"
echo "==> Detecting OS: $OS"

if [ "$OS" = "Darwin" ]; then
  echo "==> Configuring native services on macOS (Homebrew)..."

  # Ensure Homebrew and PostgreSQL binaries are in PATH
  export PATH="/opt/homebrew/bin:/usr/local/bin:/opt/homebrew/opt/postgresql@17/bin:/usr/local/opt/postgresql@17/bin:$PATH"

  # Check / install Redis
  if ! command -v redis-server >/dev/null 2>&1; then
    echo "==> Installing redis via Homebrew..."
    brew install redis
  fi

  # Start Redis if not already listening
  if ! redis-cli -h localhost -p 6379 ping >/dev/null 2>&1; then
    echo "==> Starting Redis service..."
    brew services start redis || redis-server --daemonize yes
  fi

  # Check / install PostgreSQL
  if ! command -v psql >/dev/null 2>&1; then
    echo "==> Installing postgresql@17 via Homebrew..."
    brew install postgresql@17
  fi

  PG_DATA_DIR=""
  for candidate in "/opt/homebrew/var/postgresql@17" "/usr/local/var/postgresql@17" "/opt/homebrew/var/postgres" "/usr/local/var/postgres"; do
    if [ -d "$candidate" ]; then
      PG_DATA_DIR="$candidate"
      break
    fi
  done

  if [ -n "$PG_DATA_DIR" ] && [ -f "$PG_DATA_DIR/postgresql.conf" ]; then
    # Ensure TCP listening is enabled
    if grep -q "#listen_addresses = 'localhost'" "$PG_DATA_DIR/postgresql.conf"; then
      sed -i '' "s/#listen_addresses = 'localhost'/listen_addresses = '*'/g" "$PG_DATA_DIR/postgresql.conf" || true
    fi
  fi

  # Start PostgreSQL if not already accepting connections
  if ! pg_isready -h localhost -p 5432 >/dev/null 2>&1; then
    echo "==> Starting PostgreSQL service..."
    brew services start postgresql@17 || ( [ -n "$PG_DATA_DIR" ] && pg_ctl -D "$PG_DATA_DIR" -l "$PG_DATA_DIR/server.log" start ) || true
  fi

  # Wait for PostgreSQL to be ready (up to 20 seconds)
  echo "==> Waiting for PostgreSQL to accept connections..."
  ready=0
  for i in $(seq 1 20); do
    if pg_isready -h localhost -p 5432 >/dev/null 2>&1 || pg_isready -p 5432 >/dev/null 2>&1; then
      ready=1
      break
    fi
    sleep 1
  done

  if [ "$ready" -ne 1 ]; then
    echo "warning: PostgreSQL is not ready yet on localhost:5432"
  else
    echo "==> PostgreSQL is ready."
  fi

  # Ensure user postgres and test database exist
  psql -h localhost -d postgres -c "DO \$\$ BEGIN IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'postgres') THEN CREATE ROLE postgres WITH SUPERUSER LOGIN PASSWORD 'postgres'; ELSE ALTER ROLE postgres WITH SUPERUSER LOGIN PASSWORD 'postgres'; END IF; END \$\$;" 2>/dev/null || \
  psql -d postgres -c "DO \$\$ BEGIN IF NOT EXISTS (SELECT FROM pg_roles WHERE rolname = 'postgres') THEN CREATE ROLE postgres WITH SUPERUSER LOGIN PASSWORD 'postgres'; ELSE ALTER ROLE postgres WITH SUPERUSER LOGIN PASSWORD 'postgres'; END IF; END \$\$;" 2>/dev/null || true

  PGPASSWORD=postgres psql -h localhost -U postgres -d postgres -c "CREATE DATABASE nvbes_identity_test OWNER postgres;" 2>/dev/null || true
  PGPASSWORD=postgres psql -h localhost -U postgres -d postgres -c "CREATE DATABASE nvbes_test OWNER postgres;" 2>/dev/null || true

elif [ "$OS" = "Linux" ]; then
  echo "==> Configuring services on Linux..."

  # If running in GitHub Actions with container services, wait for them
  echo "==> Waiting for PostgreSQL and Redis to be ready..."
  for i in $(seq 1 30); do
    if command -v pg_isready >/dev/null 2>&1 && pg_isready -h 127.0.0.1 -p 5432 >/dev/null 2>&1; then
      break
    fi
    sleep 1
  done

  if command -v psql >/dev/null 2>&1; then
    PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d postgres -c "CREATE DATABASE nvbes_identity_test OWNER postgres;" 2>/dev/null || true
    PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d postgres -c "CREATE DATABASE nvbes_test OWNER postgres;" 2>/dev/null || true
  fi

else
  echo "==> Unknown OS: $OS (skipping native service configuration)"
fi

echo "==> Service setup completed."
