# Docs Update Workflow

## Trigger

Use for instructions, memory, skills, workflows, research docs, ADRs, and
tool-specific prompt surfaces.

## Steps

1. Identify the canonical source file.
2. Update canonical docs first.
3. Update derived surfaces through scripts when possible.
4. Keep memory durable and concise.
5. Run `pnpm agent:doctor`.

## Output Contract

- Canonical files changed.
- Derived files changed.
- Validation result.
- Any manual follow-up.
