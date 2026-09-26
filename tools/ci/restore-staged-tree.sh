#!/usr/bin/env bash
# Restores the index from NVBES_STAGED_TREE after long pre-commit hooks.
# Guards against concurrent IDE/fsmonitor/fetch races that clear the index
# mid-hook and produce empty commits.
set -euo pipefail

tree="${NVBES_STAGED_TREE:-}"
if [[ -z "$tree" ]]; then
  echo "restore-staged-tree: skip (NVBES_STAGED_TREE unset)"
  exit 0
fi

if ! git cat-file -t "$tree" >/dev/null 2>&1; then
  echo "restore-staged-tree: BLOCKED unknown tree $tree" >&2
  exit 1
fi

before="$(git diff --cached --name-only | wc -l | tr -d ' ')"
git read-tree "$tree"
after="$(git diff --cached --name-only | wc -l | tr -d ' ')"
got="$(git write-tree)"

if [[ "$got" != "$tree" ]]; then
  echo "restore-staged-tree: BLOCKED write-tree $got != saved $tree" >&2
  exit 1
fi

if [[ "$after" -eq 0 ]]; then
  echo "restore-staged-tree: BLOCKED restored tree has 0 staged paths" >&2
  exit 1
fi

echo "restore-staged-tree: ok (staged $before → $after, tree $tree)"
