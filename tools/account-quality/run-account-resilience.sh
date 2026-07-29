#!/usr/bin/env bash

set -euo pipefail

profile="${K6_PROFILE:?K6_PROFILE is required}"
suite="${K6_SUITE:?K6_SUITE is required}"

node "$(dirname "${BASH_SOURCE[0]}")/validate-resilience-inputs.mjs"

if [[ "$profile" == "scalability" ]]; then
  exec node "$(dirname "${BASH_SOURCE[0]}")/check-scalability-comparison.mjs"
fi

if [[ "${ACCOUNT_K6_DURATION:-}" == "profile-default" ]]; then
  unset ACCOUNT_K6_DURATION
fi

exec bash "$(dirname "${BASH_SOURCE[0]}")/run-account-k6.sh"
