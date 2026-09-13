#!/usr/bin/env bash
set -euo pipefail

OS="$(uname -s)"
echo "==> Setting up test services (OS: $OS)..."

# Ensure Homebrew and PostgreSQL binaries are in PATH if on macOS
if [ "$OS" = "Darwin" ]; then
  export PATH="/opt/homebrew/bin:/usr/local/bin:/opt/homebrew/opt/postgresql@17/bin:/usr/local/opt/postgresql@17/bin:$PATH"
fi

probe_port() {
  local host="$1"
  local port="$2"
  if command -v node >/dev/null 2>&1; then
    node -e "
      const net = require('net');
      const s = net.createConnection({ port: $port, host: '$host' }, () => { s.destroy(); process.exit(0); });
      s.on('error', () => process.exit(1));
      s.setTimeout(800, () => { s.destroy(); process.exit(1); });
    " >/dev/null 2>&1
  elif command -v nc >/dev/null 2>&1; then
    nc -z "$host" "$port" >/dev/null 2>&1
  else
    (echo > "/dev/tcp/$host/$port") >/dev/null 2>&1
  fi
}

# 2. Check/Wait for PostgreSQL
echo "==> Checking PostgreSQL availability on 127.0.0.1:5432..."
pg_ready=0
for i in $(seq 1 15); do
  if probe_port 127.0.0.1 5432 || probe_port localhost 5432; then
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
    if probe_port 127.0.0.1 5432 || probe_port localhost 5432; then
      pg_ready=1
      break
    fi
    sleep 1
  done
fi

if [ "$pg_ready" -eq 1 ]; then
  echo "==> PostgreSQL is ready on port 5432."
  
  # Ensure superuser postgres and test databases exist if psql is present
  if command -v psql >/dev/null 2>&1; then
    PGCONNECT_TIMEOUT=2 PGPASSWORD=postgres psql -w -h 127.0.0.1 -U postgres -d postgres -c "CREATE DATABASE nvbes_identity_test OWNER postgres;" 2>/dev/null || \
    PGCONNECT_TIMEOUT=2 psql -w -h 127.0.0.1 -d postgres -c "CREATE DATABASE nvbes_identity_test OWNER postgres;" 2>/dev/null || true

    PGCONNECT_TIMEOUT=2 PGPASSWORD=postgres psql -w -h 127.0.0.1 -U postgres -d postgres -c "CREATE DATABASE nvbes_test OWNER postgres;" 2>/dev/null || \
    PGCONNECT_TIMEOUT=2 psql -w -h 127.0.0.1 -d postgres -c "CREATE DATABASE nvbes_test OWNER postgres;" 2>/dev/null || true
  fi
else
  echo "warning: PostgreSQL is not responding on port 5432"
fi

echo "==> Service setup completed."
