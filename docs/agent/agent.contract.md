# Agent Contract

This file is the canonical agent contract for nvbes. Tool-specific instruction
files must be generated from `docs/agent/*` and must not become independent
sources of truth.

## Operating Rules

- Read `AGENTS.md` before changing the repository.
- Treat `docs/product/nvbes-product-strategy.md` as the active V1 product
  authority. Cloud/Drive, B2B, Enterprise, and advanced compliance are not V1
  implementation scope.
- Keep the whole project's recurring cost at or below EUR 30 including taxes;
  FinOps is a design and delivery gate.
- Default to safe manual administration and moderation while there is one
  operator. Automate only a measured need with proven security and budget fit.
- Prefer short, flat, self-documenting files.
- Keep Rust files in `src/` with dot-qualified names and explicit `#[path]`
  module wiring.
- Keep TypeScript free of `any`; use precise types or `unknown`.
- Do not introduce known technical or structural debt in the delivered V1
  scope. After V1, record any accepted debt with an owner, measured impact,
  review date, and resolution condition.
- Do not rename or move files outside the requested scope.
- Use Nx to reduce context when project boundaries or affected scope matter.
- Use `rtk` as the shell command prefix.
- Use ripgrep for local discovery: `rg` for content searches and `rg --files`
  for file discovery. Fall back to `grep` or `find` only when `rg` is
  unavailable or unsuitable.
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
