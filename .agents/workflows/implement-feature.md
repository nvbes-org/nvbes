# Implement Feature Workflow

## Trigger

Use for a scoped feature or behavior change.

## Inputs

- User request.
- Relevant profile.
- Nearest project instructions.
- Existing code and tests around the target behavior.

## Steps

1. Discover affected project and files.
2. Read the nearest related implementation and tests.
3. Identify existing patterns.
4. Make scoped edits.
5. Add or update targeted tests when business behavior changes.
6. Run required checks.
7. Report changed files and validation.

## Validation

- Rust: `cargo check --workspace`.
- Web: targeted package check or `pnpm check:web`.
- Agent-system changes: `pnpm agent:doctor`.
