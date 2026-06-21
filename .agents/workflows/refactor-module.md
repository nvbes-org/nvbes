# Refactor Module Workflow

## Trigger

Use for bounded redesign, file splitting, or removal of weak structure.

## Rules

- Refactor only the named scope.
- Preserve external behavior unless the user requested behavior changes.
- Prefer clearer boundaries over line movement.
- Do not add temporary compatibility layers unless explicitly justified.

## Steps

1. Map current responsibilities.
2. Identify the target boundary.
3. Plan file/module changes.
4. Apply edits in small coherent groups.
5. Update tests and imports.
6. Run required checks.
7. Document durable design decisions if needed.
