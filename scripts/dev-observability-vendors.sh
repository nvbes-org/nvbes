#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENDOR_DIR="${NVBES_OBSERVABILITY_VENDOR_DIR:-$ROOT_DIR/.devcontainer/.vendor/observability}"
SENTRY_DIR="$VENDOR_DIR/sentry-self-hosted"
POSTHOG_DIR="$VENDOR_DIR/posthog-hobby"
SENTRY_BIND="${NVBES_SENTRY_BIND:-127.0.0.1:19000}"
SENTRY_PROJECT="${NVBES_SENTRY_COMPOSE_PROJECT:-nvbes_sentry}"
POSTHOG_PROJECT="${NVBES_POSTHOG_COMPOSE_PROJECT:-nvbes_posthog}"

usage() {
  cat <<'EOF'
Usage: scripts/dev-observability-vendors.sh <command>

Commands:
  sentry-install       Clone latest Sentry self-hosted release and run install.sh.
  sentry-up            Start the local Sentry stack.
  sentry-down          Stop the local Sentry stack.
  sentry-create-user   Create a Sentry user in the local stack.
  posthog-install      Run PostHog's upstream hobby installer in the vendor dir.
  posthog-up           Start the local PostHog hobby stack after installation.
  posthog-down         Stop the local PostHog hobby stack.
  status               Show local vendor stack status.

PostHog install requires:
  NVBES_POSTHOG_LOCAL_DOMAIN=<domain>
  NVBES_ACCEPT_POSTHOG_HOBBY_INSTALLER=1
EOF
}

require_docker() {
  if ! command -v docker >/dev/null 2>&1; then
    printf 'Docker CLI is required. Rebuild the Dev Container so docker-outside-of-docker is installed.\n' >&2
    exit 1
  fi

  if ! docker info >/dev/null 2>&1; then
    printf 'Docker is not reachable from this shell. Check Docker Desktop or the Dev Container Docker socket.\n' >&2
    exit 1
  fi
}

docker_host_path() {
  local container_path="$1"
  local root_source
  local relative_path

  if [ "${NVBES_DEVCONTAINER:-}" != "true" ]; then
    printf '%s\n' "$container_path"
    return 0
  fi

  root_source="$(
    docker inspect "$HOSTNAME" \
      --format '{{range .Mounts}}{{if eq .Destination "/workspaces/nvbes"}}{{.Source}}{{end}}{{end}}' \
      2>/dev/null || true
  )"

  if [ -z "$root_source" ]; then
    printf '%s\n' "$container_path"
    return 0
  fi

  relative_path="${container_path#"$ROOT_DIR"}"
  printf '%s%s\n' "$root_source" "$relative_path"
}

with_sentry_compose_host_paths() {
  local host_sentry_dir="$1"
  local docker_bin
  shift

  if [ "$host_sentry_dir" = "$SENTRY_DIR" ]; then
    "$@"
    return
  fi

  local temp_bin
  local override_file
  local status
  docker_bin="$(command -v docker)"
  temp_bin="$(mktemp -d)"
  override_file="$temp_bin/build-contexts.yml"

  "$docker_bin" compose \
    --env-file "$SENTRY_DIR/.env" \
    --file "$SENTRY_DIR/docker-compose.yml" \
    --project-directory "$SENTRY_DIR" \
    config --format json \
    | python3 -c '
import json
import sys

data = json.load(sys.stdin)
services = {
    name: service["build"]["context"]
    for name, service in data.get("services", {}).items()
    if isinstance(service.get("build"), dict) and service["build"].get("context")
}

print("services:")
for name in sorted(services):
    print(f"  {json.dumps(name)}:")
    print("    build:")
    print(f"      context: {json.dumps(services[name])}")
    ' >"$override_file"

  cat >"$temp_bin/docker" <<EOF
#!/usr/bin/env bash
if [ "\${1:-}" = "compose" ]; then
  shift
  exec "$docker_bin" compose \\
    --file "$SENTRY_DIR/docker-compose.yml" \\
    --file "$override_file" \\
    --project-directory "$host_sentry_dir" \\
    "\$@"
fi
exec "$docker_bin" "\$@"
EOF
  chmod +x "$temp_bin/docker"

  set +e
  PATH="$temp_bin:$PATH" "$@"
  status=$?
  set -e
  rm -rf "$temp_bin"
  return "$status"
}

set_env_var() {
  local file="$1"
  local key="$2"
  local value="$3"
  local tmp_file

  tmp_file="$(mktemp)"
  touch "$file"
  awk -v key="$key" -v value="$value" '
    BEGIN { found = 0 }
    $0 ~ "^" key "=" {
      print key "=" value
      found = 1
      next
    }
    { print }
    END {
      if (found == 0) {
        print key "=" value
      }
    }
  ' "$file" > "$tmp_file"
  mv "$tmp_file" "$file"
}

latest_sentry_release() {
  curl -fsSL -o /dev/null -w '%{url_effective}' https://github.com/getsentry/self-hosted/releases/latest \
    | sed 's#.*/##'
}

prepare_vendor_dir() {
  mkdir -p "$VENDOR_DIR"
}

sentry_install() {
  require_docker
  prepare_vendor_dir

  local version
  local host_sentry_dir
  version="${NVBES_SENTRY_SELF_HOSTED_VERSION:-$(latest_sentry_release)}"

  if [ ! -d "$SENTRY_DIR/.git" ]; then
    git clone https://github.com/getsentry/self-hosted.git "$SENTRY_DIR"
  fi

  git -C "$SENTRY_DIR" fetch --tags --force
  git -C "$SENTRY_DIR" checkout "$version"

  set_env_var "$SENTRY_DIR/.env" "COMPOSE_PROJECT_NAME" "$SENTRY_PROJECT"
  set_env_var "$SENTRY_DIR/.env" "SENTRY_BIND" "$SENTRY_BIND"
  set_env_var "$SENTRY_DIR/.env" "SENTRY_EVENT_RETENTION_DAYS" "${NVBES_SENTRY_EVENT_RETENTION_DAYS:-7}"

  (
    cd "$SENTRY_DIR"
    host_sentry_dir="$(docker_host_path "$SENTRY_DIR")"
    with_sentry_compose_host_paths "$host_sentry_dir" \
      ./install.sh --skip-user-creation --no-report-self-hosted-issues
  )

  printf 'Sentry installed in %s\n' "$SENTRY_DIR"
  printf 'Start it with: pnpm dev:obs:sentry:up\n'
}

sentry_compose() {
  if [ ! -f "$SENTRY_DIR/docker-compose.yml" ]; then
    printf 'Sentry is not installed yet. Run: pnpm dev:obs:sentry:install\n' >&2
    exit 1
  fi

  (
    cd "$SENTRY_DIR"
    COMPOSE_PROJECT_NAME="$SENTRY_PROJECT" docker compose \
      --env-file "$SENTRY_DIR/.env" \
      --file "$SENTRY_DIR/docker-compose.yml" \
      --project-directory "$(docker_host_path "$SENTRY_DIR")" \
      "$@"
  )
}

sentry_up() {
  require_docker
  sentry_compose up --wait -d
  printf 'Sentry local: http://127.0.0.1:%s\n' "${SENTRY_BIND##*:}"
}

sentry_down() {
  require_docker
  sentry_compose down
}

sentry_create_user() {
  require_docker
  sentry_compose run --rm web createuser
}

posthog_install() {
  require_docker
  prepare_vendor_dir

  if [ "${NVBES_ACCEPT_POSTHOG_HOBBY_INSTALLER:-}" != "1" ]; then
    printf 'Refusing to run PostHog upstream hobby installer without explicit opt-in.\n' >&2
    printf 'Set NVBES_ACCEPT_POSTHOG_HOBBY_INSTALLER=1 after reading docs/development/devcontainer.md.\n' >&2
    exit 1
  fi

  if [ -z "${NVBES_POSTHOG_LOCAL_DOMAIN:-}" ]; then
    printf 'Set NVBES_POSTHOG_LOCAL_DOMAIN to the domain handled by the PostHog hobby installer.\n' >&2
    exit 1
  fi

  mkdir -p "$POSTHOG_DIR"
  curl -fsSL https://raw.githubusercontent.com/posthog/posthog/HEAD/bin/deploy-hobby \
    -o "$POSTHOG_DIR/deploy-hobby"
  chmod +x "$POSTHOG_DIR/deploy-hobby"

  (
    cd "$POSTHOG_DIR"
    SKIP_HEALTH_CHECK="${SKIP_HEALTH_CHECK:-1}" \
      POSTHOG_APP_TAG="${NVBES_POSTHOG_APP_TAG:-latest}" \
      "$POSTHOG_DIR/deploy-hobby" "${NVBES_POSTHOG_APP_TAG:-latest}" "$NVBES_POSTHOG_LOCAL_DOMAIN"
  )

  printf 'PostHog hobby installer ran in %s\n' "$POSTHOG_DIR"
}

posthog_compose() {
  if [ ! -f "$POSTHOG_DIR/docker-compose.yml" ]; then
    printf 'PostHog is not installed yet. Run: pnpm dev:obs:posthog:install\n' >&2
    exit 1
  fi

  (
    cd "$POSTHOG_DIR"
    COMPOSE_PROJECT_NAME="$POSTHOG_PROJECT" docker compose "$@"
  )
}

posthog_up() {
  require_docker
  posthog_compose up -d
}

posthog_down() {
  require_docker
  posthog_compose down
}

status() {
  require_docker
  printf 'Sentry containers:\n'
  docker ps \
    --filter "label=com.docker.compose.project=$SENTRY_PROJECT" \
    --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'
  printf '\nPostHog containers:\n'
  docker ps \
    --filter "label=com.docker.compose.project=$POSTHOG_PROJECT" \
    --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'
}

case "${1:-}" in
  sentry-install) sentry_install ;;
  sentry-up) sentry_up ;;
  sentry-down) sentry_down ;;
  sentry-create-user) sentry_create_user ;;
  posthog-install) posthog_install ;;
  posthog-up) posthog_up ;;
  posthog-down) posthog_down ;;
  status) status ;;
  -h|--help|help|"") usage ;;
  *)
    usage >&2
    exit 1
    ;;
esac
