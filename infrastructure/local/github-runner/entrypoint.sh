#!/bin/bash
set -e

if [[ -e /var/run/docker.sock ]]; then
  sudo chmod 666 /var/run/docker.sock || true
fi
sudo chown -R runner:runner /usr/local/cargo /home/runner/.local 2>/dev/null || true

if [[ -z "$GITHUB_REPOSITORY_URL" || -z "$RUNNER_TOKEN" ]]; then
  echo "Error: GITHUB_REPOSITORY_URL and RUNNER_TOKEN environment variables are required."
  exit 1
fi

RUNNER_NAME="nvbes-docker-$(hostname)-${RANDOM}"
RUNNER_LABELS="${RUNNER_LABELS:-self-hosted,linux,ARM64,docker,macOS}"

if [[ ! -f .runner ]]; then
  echo "Configuring runner $RUNNER_NAME for $GITHUB_REPOSITORY_URL with labels $RUNNER_LABELS..."
  ./config.sh \
    --url "$GITHUB_REPOSITORY_URL" \
    --token "$RUNNER_TOKEN" \
    --name "$RUNNER_NAME" \
    --labels "$RUNNER_LABELS" \
    --unattended \
    --replace
fi

cleanup() {
  echo "Deregistering runner..."
  ./config.sh remove --token "$RUNNER_TOKEN" || true
}

trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM

echo "Starting runner..."
./run.sh & wait $!
