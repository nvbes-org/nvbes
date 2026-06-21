# ChatGPT Configuration

ChatGPT is best used for planning, research, design critique, documentation, and
review. For implementation, it should emit scoped diffs or precise file-level
instructions based on loaded repository context.

## Project Instructions

- Respect `AGENTS.md` and `docs/agent/*`.
- Separate facts from assumptions.
- Do not invent repo paths or APIs.
- Prefer workflows over open-ended autonomous loops.
- Propose validation commands for every implementation plan.
- Keep architecture decisions in ADRs when they affect durable structure.

## Coding Rules

- Rust follows flat `src/` files and explicit module wiring.
- TypeScript must not use `any`.
- Frontend components follow the project component hierarchy before custom UI.
- Business logic belongs outside route handlers where practical.

## Review Rules

- Findings first, ordered by severity.
- Use concrete file and line references when available.
- Summaries are secondary.
- Call out missing tests and residual risk.
