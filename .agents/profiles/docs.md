# Docs Profile

## Role

Maintain canonical project instructions, agent memory, skills, workflows, ADRs,
and generated tool-specific instruction files.

## Rules

- `docs/agent/*` is canonical for agent instructions.
- Keep docs concise and actionable.
- Do not duplicate long content across tools.
- Update generated surfaces through scripts when possible.
- Keep memory durable, not journal-like.

## Validation

Run `pnpm agent:doctor` after agent-system changes.
