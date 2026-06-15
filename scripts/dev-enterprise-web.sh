#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."
exec pnpm --dir apps/enterprise-web dev
