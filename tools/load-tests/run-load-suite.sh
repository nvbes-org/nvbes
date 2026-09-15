#!/usr/bin/env bash

set -euo pipefail
umask 077

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
profile="${K6_PROFILE:?K6_PROFILE is required (smoke, load, spike, stress, soak, volume, scalability)}"
service="${K6_SERVICE:-identity}"
suite="${K6_SUITE:-${service}-api}"
target_environment="${NVBES_TARGET_ENV:?NVBES_TARGET_ENV is required (local, development, staging, test, ci)}"

case "$profile" in
  smoke | load | volume | spike | stress | soak | scalability) ;;
  *) echo "Unsupported K6_PROFILE=$profile" >&2; exit 2 ;;
esac

case "$target_environment" in
  ci | development | local | staging | test) ;;
  *) echo "Load tests require an explicit non-production NVBES_TARGET_ENV." >&2; exit 2 ;;
esac

for reserved_variable in K6_DURATION K6_ITERATIONS K6_VUS; do
  if [[ -n "${!reserved_variable:-}" ]]; then
    echo "$reserved_variable is reserved by k6 and must be unset." >&2
    exit 2
  fi
done

results_dir="${K6_RESULTS_DIR:-$workspace_root/.temp/load-tests/k6}"
mkdir -p "$results_dir"
summary_name="${K6_SUMMARY_NAME:-${suite}-${profile}.json}"
summary_path="$results_dir/$summary_name"

script_path="tools/load-tests/${service}/${suite}.js"
if [[ ! -f "$workspace_root/$script_path" ]]; then
  echo "Load test script $script_path does not exist." >&2
  exit 2
fi

run_status=0
if command -v k6 >/dev/null 2>&1; then
  echo "Executing k6 test with local binary: $script_path ($profile)"
  K6_SUMMARY_EXPORT="$summary_path" \
    k6 run "$workspace_root/$script_path" || run_status=$?
else
  if ! command -v docker >/dev/null 2>&1; then
    echo "k6 or Docker is required to execute load tests." >&2
    exit 2
  fi

  echo "Executing k6 test with pinned container: $script_path ($profile)"
  k6_image="grafana/k6:2.0.0@sha256:a33a0cfdc4d2483d6b7a3a22e726a499ff2831a671a49239104cd34a9937523c"
  docker run \
    --rm \
    --user "$(id -u):$(id -g)" \
    --add-host host.docker.internal:host-gateway \
    --volume "$workspace_root:/workspace:ro" \
    --volume "$results_dir:/results" \
    --workdir /workspace \
    --env "K6_PROFILE=$profile" \
    --env "NVBES_TARGET_ENV=$target_environment" \
    --env "K6_SUMMARY_EXPORT=/results/$summary_name" \
    --env IDENTITY_SERVICE_BASE_URL \
    --env BILLING_SERVICE_BASE_URL \
    --env ACCOUNT_SERVICE_BASE_URL \
    --env IDENTITY_LOAD_ALLOWED_ORIGINS \
    --env BILLING_LOAD_ALLOWED_ORIGINS \
    --env ACCOUNT_LOAD_ALLOWED_ORIGINS \
    --env IDENTITY_PRODUCTION_DENIED_ORIGINS \
    --env BILLING_PRODUCTION_DENIED_ORIGINS \
    --env ACCOUNT_PRODUCTION_DENIED_ORIGINS \
    "$k6_image" run "/workspace/$script_path" || run_status=$?
fi

if [[ $run_status -ne 0 ]]; then
  echo "k6 load test failed with exit code $run_status" >&2
  exit "$run_status"
fi

echo "k6 $profile suite $suite finished successfully. Report saved to $summary_path"
