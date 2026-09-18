#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."
# No dotenv loading, service provisioning, or package installation in this lane.
exec pnpm --config.verify-deps-before-run=false exec nx run rust-workspace:micro-test
