#!/usr/bin/env bash
set -euo pipefail

echo "Installing Vite+ (workspace) and workspace dependencies..."
pnpm -w add -D vite-plus
pnpm -w install

echo "Installing lefthook and registering hooks..."
pnpm dlx lefthook install

echo "Done. You can now run 'vp check', 'pnpm format', or 'pnpm lint' at the repo root."
