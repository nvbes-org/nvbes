#!/usr/bin/env bash
set -euo pipefail

echo "==> [1/5] Resetting Nx local cache..."
pnpm exec nx reset || true

echo "==> [2/5] Cleaning dangling Docker containers and images..."
if command -v docker >/dev/null 2>&1; then
  docker image prune -f || true
  docker builder prune -f --reserved-space 20GB 2>/dev/null || docker builder prune -f || true
fi

echo "==> [3/5] Cleaning local test & coverage artifacts..."
rm -rf coverage/ test-results/ playwright-report/ .temp/
find apps libs -name "dist" -type d -prune -exec rm -rf {} + 2>/dev/null || true

echo "==> [4/5] Checking Cargo target directory size..."
if [[ -d "target" ]]; then
  target_size=$(du -sh target | cut -f1)
  echo "    Current target/ size: $target_size"
fi

echo "==> [5/5] Available disk space:"
df -h /System/Volumes/Data 2>/dev/null || df -h .

echo "Cache cleanup completed successfully."
