#!/usr/bin/env bash

set -euo pipefail

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$workspace_root"

toolchain="nightly-2026-09-09"
output_root=".temp/v1-rust"
llvm_report="$output_root/llvm.json"
mutation_report="$output_root/mutants.out/outcomes.json"
mutation_status=0

for command in cargo rustup node; do
  if ! command -v "$command" >/dev/null 2>&1; then
    printf 'error: missing required command: %s\n' "$command" >&2
    exit 1
  fi
done
rustup run "$toolchain" -- cargo llvm-cov --version >/dev/null
if [ "$(cargo mutants --version)" != "cargo-mutants 27.1.0" ]; then
  printf 'error: cargo-mutants 27.1.0 is required\n' >&2
  exit 1
fi

mapfile -t packages < <(node --input-type=module -e '
  import { loadV1 } from "./tools/test-summary/v1-context.mjs";
  import { productionUnits } from "./tools/test-summary/v1-catalogue.mjs";
  const context = loadV1(process.cwd());
  for (const unit of productionUnits(context.root, context.domains, process.cwd())) {
    if (unit.language === "rust") console.log(unit.name);
  }
')
if [ "${#packages[@]}" -eq 0 ]; then
  printf 'error: V1 Rust catalogue is empty\n' >&2
  exit 1
fi

mkdir -p "$output_root"
rustup run "$toolchain" -- cargo llvm-cov clean --workspace
rustup run "$toolchain" -- cargo llvm-cov \
  --workspace \
  --all-targets \
  --branch \
  --locked \
  --ignore-filename-regex '(\.tests\.rs|\.test_support\.rs)$' \
  --json \
  --output-path "$llvm_report"

package_args=()
for package in "${packages[@]}"; do
  package_args+=(--package "$package")
done
set +e
cargo mutants \
  "${package_args[@]}" \
  --timeout "${NVBES_MUTATION_TIMEOUT:-600}" \
  --jobs "${NVBES_MUTATION_JOBS:-4}" \
  --no-times \
  -o "$output_root"
mutation_status=$?
set -e
if [ "$mutation_status" -ne 0 ] && [ "$mutation_status" -ne 2 ] && [ "$mutation_status" -ne 3 ]; then
  printf 'error: cargo-mutants failed with exit code %s\n' "$mutation_status" >&2
  exit "$mutation_status"
fi
if [ ! -s "$mutation_report" ]; then
  printf 'error: cargo-mutants did not produce %s\n' "$mutation_report" >&2
  exit 1
fi
