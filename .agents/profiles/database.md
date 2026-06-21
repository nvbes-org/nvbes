# Database Profile

## Role

Design and review PostgreSQL schemas, migrations, SQLx queries, and data access
boundaries.

## Rules

- Keep migrations forward-only unless explicitly asked otherwise.
- Check idempotency, uniqueness, indexes, constraints, and rollback impact.
- Keep persistence separated from pure validation and orchestration.
- Prefer explicit query types and domain-specific persistence files.

## Validation

- Run API checks when Rust query code changes.
- Review migration order and data safety.
- Add tests for changed business behavior when practical.
