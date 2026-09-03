# Platform Operations Cockpit Service

Minimal solo-operator cockpit for nvbes V1.

## Architecture

- **Domain**: Platform Operations (bounded context)
- **Role**: Operator safety, aggregated runtime health, jobs/outbox monitor, FinOps ceiling compliance, manual billing reconciliation, email status, shadow trust-risk reviews, degraded modes, audited mutations.
- **Constraints**: Read-only by default. Automated enforcement strictly forbidden. Mutations require operator identity, mandatory reason (3-300 chars), and idempotency key.
- **Runtime**: Scaleway Serverless Container with `min_scale = 0`, `max_scale = 1`.

## Local Development

```bash
cargo run --bin nvbes-platform-operations
```

The cockpit serves `/health/live`, `/health/ready`, and protected routes on port 8084.
