# V2 Debt Register

## Status

- `total_rows`: 1
- `v1_required_rows`: 1
- `open_v1_required_rows`: 1
- `no_go_rows`: 1

## Rules

- `V1 Required: yes` means the item is required for the first production cutover.
- A V1-required row can reach `decision: go` only when `status: closed`.
- `decision: go` requires concrete audit, backlog, query, report, review, ticket, evidence or acceptance proof.
- `open` rows must remain `no-go`.
- The migration remains incomplete until every V1-required debt item is closed or removed from scope by owner acceptance.

## Verification

```bash
pnpm check:migration-v2-debt -- --strict
```

## Debt Review

| Item | Owner | Scope | V1 Required | Evidence | Status | Decision |
|---|---|---|---|---|---|---|
| zero-debt cutover review | Migration lead | V1 | yes | pending debt review report, backlog query result and product owner acceptance evidence | open | no-go |

## Decision

`no-go`: the V2 debt review has not yet produced accepted evidence proving that no V1-required debt remains.
