# Platform Operations Cockpit Service

Manual solo-operator API for nvbes V1. See the
[runtime, commands and proof guide](../../docs/operations/platform-operations-api.md).

## Architecture

- **Domain**: Platform Operations (bounded context)
- **Role**: Durable cases, manual domain observations, append-only audit, idempotent case commands and EUR TTC cost ledger. Domain mutations are unavailable until their secured adapters exist.
- **Constraints**: Read-only by default. Automated enforcement strictly forbidden. Mutations require operator identity, mandatory reason (3-300 chars), and idempotency key.
- **Runtime**: Scaleway Serverless Container with `min_scale = 0`, `max_scale = 1`.

## Local Development

```bash
# Configure the dedicated Operations database and trusted operator JWT issuer first.
cargo run --bin nvbes-platform-operations -- migrate
cargo run --bin nvbes-platform-operations
```

The cockpit serves `/health/live`, `/health/ready`, and protected routes on port 8084.
