#!/usr/bin/env bash

set -euo pipefail
umask 077

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
profile="${K6_PROFILE:?K6_PROFILE is required}"
suite="${K6_SUITE:-account-api}"
target_environment="${NVBES_TARGET_ENV:?NVBES_TARGET_ENV is required}"
service_url="${ACCOUNT_SERVICE_BASE_URL:?ACCOUNT_SERVICE_BASE_URL is required}"

case "$profile" in
  smoke | load | volume | spike | stress | soak | scalability) ;;
  *) echo "Unsupported K6_PROFILE=$profile" >&2; exit 2 ;;
esac

case "$suite" in
  account-api | account-session) ;;
  *) echo "Unsupported K6_SUITE=$suite" >&2; exit 2 ;;
esac

case "$target_environment" in
  ci | development | local | staging | test) ;;
  *) echo "Load tests require an explicit non-production NVBES_TARGET_ENV." >&2; exit 2 ;;
esac

for reserved_variable in K6_DURATION K6_ITERATIONS K6_VUS; do
  if [[ -n "${!reserved_variable:-}" ]]; then
    echo "$reserved_variable is reserved by k6 and must be unset; use ACCOUNT_K6_*." >&2
    exit 2
  fi
done

node "$workspace_root/tools/account-quality/validate-load-target.mjs" service

if [[ "$suite" == "account-session" && -z "${ACCOUNT_TEST_COOKIE:-}" ]]; then
  echo "ACCOUNT_TEST_COOKIE is required for account-session." >&2
  exit 2
fi
if [[ "$suite" == "account-session" && "${#ACCOUNT_TEST_COOKIE}" -lt 16 ]]; then
  echo "ACCOUNT_TEST_COOKIE must be a synthetic credential of at least 16 characters." >&2
  exit 2
fi
if [[ "$suite" == "account-session" && ( "$ACCOUNT_TEST_COOKIE" == *$'\n'* || "$ACCOUNT_TEST_COOKIE" == *$'\r'* ) ]]; then
  echo "ACCOUNT_TEST_COOKIE must not contain control characters." >&2
  exit 2
fi
if [[ "$suite" == "account-session" && ! "${ACCOUNT_TEST_AUTHUSER:-0}" =~ ^[0-9]+$ ]]; then
  echo "ACCOUNT_TEST_AUTHUSER must be a non-negative account index." >&2
  exit 2
fi

results_dir="${K6_RESULTS_DIR:-$workspace_root/.temp/account-quality/k6}"
mkdir -p "$results_dir"
summary_name="${K6_SUMMARY_NAME:-${suite}-${profile}.json}"
if [[ ! "$summary_name" =~ ^[a-zA-Z0-9][a-zA-Z0-9._-]*\.json$ ]]; then
  echo "K6_SUMMARY_NAME must be a safe JSON filename." >&2
  exit 2
fi

metadata_name="${summary_name%.json}.metadata.json"
summary_path="$results_dir/$summary_name"
metadata_path="$results_dir/$metadata_name"
script_path="tools/load-tests/account/${suite}.js"

node "$workspace_root/tools/account-quality/write-load-metadata.mjs" \
  "$metadata_path" \
  "$summary_name"

run_status=0
if command -v k6 >/dev/null 2>&1; then
  K6_SUMMARY_EXPORT="$summary_path" \
    k6 run "$workspace_root/$script_path" || run_status=$?
else
  if ! command -v docker >/dev/null 2>&1; then
    echo "k6 or Docker is required to execute Account load tests." >&2
    exit 2
  fi

  k6_image="grafana/k6:2.0.0@sha256:a33a0cfdc4d2483d6b7a3a22e726a499ff2831a671a49239104cd34a9937523c"
  docker_args=(
    run
    --rm
    --user "$(id -u):$(id -g)"
    --add-host host.docker.internal:host-gateway
    --volume "$workspace_root:/workspace:ro"
    --volume "$results_dir:/results"
    --workdir /workspace
    --env "K6_SUMMARY_EXPORT=/results/$summary_name"
  )

  for variable in \
    ACCOUNT_LOAD_ALLOWED_ORIGINS \
    ACCOUNT_PRODUCTION_DENIED_ORIGINS \
    ACCOUNT_SERVICE_BASE_URL \
    ACCOUNT_K6_DATASET_ID \
    ACCOUNT_K6_DURATION \
    ACCOUNT_K6_ITERATIONS \
    ACCOUNT_K6_LOAD_RATE \
    ACCOUNT_K6_MAX_VUS \
    ACCOUNT_K6_PEAK_RATE \
    ACCOUNT_K6_PREALLOCATED_VUS \
    ACCOUNT_K6_REPLICAS \
    ACCOUNT_K6_SPIKE_RATE \
    ACCOUNT_K6_START_RATE \
    ACCOUNT_K6_THINK_TIME_SECONDS \
    ACCOUNT_K6_VUS \
    ACCOUNT_TEST_AUTHUSER \
    ACCOUNT_TEST_COOKIE \
    K6_PROFILE \
    K6_SUITE \
    NVBES_RELEASE \
    NVBES_TARGET_ENV
  do
    if [[ -n "${!variable:-}" ]]; then
      docker_args+=(--env "$variable")
    fi
  done

  docker "${docker_args[@]}" "$k6_image" run "/workspace/$script_path" || run_status=$?
fi

if [[ ! -f "$summary_path" ]]; then
  echo "k6 did not produce the required summary artifact." >&2
  if [[ "$run_status" -ne 0 ]]; then
    exit "$run_status"
  fi
  exit 1
fi

node "$workspace_root/tools/account-quality/verify-load-artifact.mjs" "$summary_path"
exit "$run_status"
