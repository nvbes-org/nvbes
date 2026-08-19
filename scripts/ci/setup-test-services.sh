#!/usr/bin/env bash
set -euo pipefail

OS="$(uname -s)"
echo "==> Setting up test services (OS: $OS)..."

# Ensure Homebrew and PostgreSQL binaries are in PATH if on macOS
if [ "$OS" = "Darwin" ]; then
  export PATH="/opt/homebrew/bin:/usr/local/bin:/opt/homebrew/opt/postgresql@17/bin:/usr/local/opt/postgresql@17/bin:$PATH"
fi

# 1. Check/Wait for Redis
echo "==> Checking Redis availability on 127.0.0.1:6379..."
redis_ready=0
for i in $(seq 1 15); do
  if command -v redis-cli >/dev/null 2>&1 && redis-cli -h 127.0.0.1 -p 6379 ping >/dev/null 2>&1; then
    redis_ready=1
    break
  fi
  sleep 1
done

if [ "$redis_ready" -ne 1 ]; then
  if [ "$OS" = "Darwin" ]; then
    echo "==> Starting native Redis via Homebrew / daemon..."
    if ! command -v redis-server >/dev/null 2>&1; then
      brew install redis || true
    fi
    brew services start redis || redis-server --daemonize yes || true
  elif [ "$OS" = "Linux" ]; then
    echo "==> Starting native Redis via system service..."
    sudo service redis-server start 2>/dev/null || sudo systemctl start redis 2>/dev/null || true
  fi
  for i in $(seq 1 10); do
    if command -v redis-cli >/dev/null 2>&1 && redis-cli -h 127.0.0.1 -p 6379 ping >/dev/null 2>&1; then
      redis_ready=1
      break
    fi
    sleep 1
  done
fi

if [ "$redis_ready" -eq 1 ]; then
  echo "==> Redis is ready on 127.0.0.1:6379."
else
  echo "warning: Redis is not responding on 127.0.0.1:6379"
fi

# 2. Check/Wait for PostgreSQL
echo "==> Checking PostgreSQL availability on 127.0.0.1:5432..."
pg_ready=0
for i in $(seq 1 20); do
  if command -v pg_isready >/dev/null 2>&1 && pg_isready -h 127.0.0.1 -p 5432 >/dev/null 2>&1; then
    pg_ready=1
    break
  fi
  sleep 1
done

if [ "$pg_ready" -ne 1 ]; then
  if [ "$OS" = "Darwin" ]; then
    echo "==> Starting native PostgreSQL via Homebrew..."
    if ! command -v psql >/dev/null 2>&1; then
      brew install postgresql@17 || true
    fi
    brew services start postgresql@17 || true
  elif [ "$OS" = "Linux" ]; then
    echo "==> Starting native PostgreSQL via system service..."
    sudo service postgresql start 2>/dev/null || sudo systemctl start postgresql 2>/dev/null || true
  fi
  for i in $(seq 1 15); do
    if command -v pg_isready >/dev/null 2>&1 && pg_isready -h 127.0.0.1 -p 5432 >/dev/null 2>&1; then
      pg_ready=1
      break
    fi
    sleep 1
  done
fi

if [ "$pg_ready" -eq 1 ]; then
  echo "==> PostgreSQL is ready on 127.0.0.1:5432."
  
  # Ensure superuser postgres and test databases exist
  if command -v psql >/dev/null 2>&1; then
    PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d postgres -c "CREATE DATABASE nvbes_identity_test OWNER postgres;" 2>/dev/null || \
    psql -h 127.0.0.1 -d postgres -c "CREATE DATABASE nvbes_identity_test OWNER postgres;" 2>/dev/null || true

    PGPASSWORD=postgres psql -h 127.0.0.1 -U postgres -d postgres -c "CREATE DATABASE nvbes_test OWNER postgres;" 2>/dev/null || \
    psql -h 127.0.0.1 -d postgres -c "CREATE DATABASE nvbes_test OWNER postgres;" 2>/dev/null || true
  fi
else
  echo "warning: PostgreSQL is not responding on 127.0.0.1:5432"
fi

echo "==> Service setup completed."
