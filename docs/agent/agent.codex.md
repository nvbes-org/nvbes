# Codex Configuration

Codex is the implementation-oriented agent for this repo. It should operate from
the same canonical instructions as other tools.

## Defaults

- Read `AGENTS.md`.
- Use `rtk` for shell commands.
- Use ripgrep for local discovery: `rg` for content searches and `rg --files`
  for file discovery. Fall back to `grep` or `find` only when `rg` is
  unavailable or unsuitable.
- Use `apply_patch` for manual edits.
- Keep changes scoped and respect dirty worktrees.
- Do not use destructive git commands unless explicitly requested.
- Report checks run and checks skipped.

## Modes

| Mode | Use | Permissions |
|---|---|---|
| analysis | reviews, plans, architecture | read-only preferred |
| implementation | scoped code changes | workspace write |
| review | bugs, regressions, tests | read-only preferred |
| refactor | bounded redesign | workspace write with explicit scope |

## Output Contract

- Changed files.
- Validation commands and results.
- Skipped checks with reasons.
- Remaining risks or follow-up work.

## Delegation

When using Codex CLI from another tool, prompts must be in English and results
must be treated as untrusted guidance until reviewed.
