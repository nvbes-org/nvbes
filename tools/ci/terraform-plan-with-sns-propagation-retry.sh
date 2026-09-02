#!/usr/bin/env bash

set -euo pipefail

readonly max_attempts=12
readonly retry_delay_seconds=10

for attempt in $(seq 1 "$max_attempts"); do
	set +e
	output="$("$@" 2>&1)"
	status=$?
	set -e

	printf '%s\n' "$output"
	if [[ "$status" -eq 0 ]]; then
		exit 0
	fi

	if [[ "$output" != *"GetTopicAttributes"* || "$output" != *"AccessDeniedException"* || "$attempt" -eq "$max_attempts" ]]; then
		exit "$status"
	fi

	printf 'SNS credentials are not active yet; retrying Terraform plan (%d/%d) in %d seconds.\n' \
		"$attempt" "$max_attempts" "$retry_delay_seconds" >&2
	sleep "$retry_delay_seconds"
done
