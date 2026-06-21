# Agent Contract

This file is the canonical agent contract for nvbes. Tool-specific instruction
files must be generated from `docs/agent/*` and must not become independent
sources of truth.

## Operating Rules

- Read `AGENTS.md` before changing the repository.
- Prefer short, flat, self-documenting files.
- Keep Rust files in `src/` with dot-qualified names and explicit `#[path]`
  module wiring.
- Keep TypeScript free of `any`; use precise types or `unknown`.
- Do not introduce temporary technical debt in touched areas.
- Do not rename or move files outside the requested scope.
- Use Nx to reduce context when project boundaries or affected scope matter.
- Use `rtk` as the shell command prefix.
- Use `rg` or `rg --files` for search.
- Use `apply_patch` for manual file edits.
- Never revert user changes unless explicitly requested.

## Validation

- Rust changes require `cargo check --workspace`.
- Frontend or TypeScript changes require the targeted package check.
- Cross-cutting changes require relevant root checks.
- Business logic changes require a targeted test or an explicit reason why no
  test applies.

## Definition Of Done

- The selected profile, workflow, and skills are appropriate for the request.
- Context was loaded from the nearest applicable instructions.
- The implementation respects product and domain boundaries.
- Required checks were run or skipped with a concrete reason.
- Durable learnings were proposed for memory only when they improve future work.
