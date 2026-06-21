# Fix Bug Workflow

## Trigger

Use when behavior is broken, a check fails, or the user reports a defect.

## Steps

1. Reproduce or inspect the failing path.
2. Isolate the root cause.
3. Make the smallest durable fix.
4. Add a regression test when practical.
5. Run the failing check again.
6. Run nearby required checks.

## Output Contract

- Root cause.
- Fix summary.
- Regression coverage.
- Validation result.
