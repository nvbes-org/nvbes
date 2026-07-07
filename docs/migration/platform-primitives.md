# Platform Primitive Ledger

## Status

- entries: 9
- active: 9
- missing: 0
- pending: 0

## Rules

- Every primitive row must match the static platform contract.
- `active` requires the evidence path to exist.
- Summary counters must match generated rows.
- Strict cutover requires every primitive control to be active.
- Generation provenance must identify write and strict cutover commands.

## Controls

| Control | Status | Proof | Evidence |
|---|---:|---|---|
| config | active | `cargo test -p nvbes-core config --locked` | `libs/rust/core/src/config.rs` |
| errors | active | `cargo test -p nvbes-core app_error_constructors_keep_standard_error_contract --locked && cargo test -p nvbes-core rate_limit_error_preserves_retry_after --locked` | `libs/rust/core/src/http.error.rs` |
| tenancy | active | `cargo test -p nvbes-tenancy --locked` | `libs/rust/tenancy/src/lib.rs` |
| audit | active | `cargo test -p nvbes-audit --locked` | `libs/rust/audit/src/lib.rs` |
| outbox | active | `cargo test -p nvbes-platform --locked` | `libs/rust/platform/src/platform.outbox.rs` |
| ports | active | `cargo test -p nvbes-ports --locked` | `libs/rust/ports/src/lib.rs` |
| observability | active | `cargo test -p nvbes-observability --locked` | `libs/rust/observability/src/lib.rs` |
| idempotency | active | `cargo test -p nvbes-core validate_key_trims_and_rejects_invalid_values --locked && cargo test -p nvbes-core request_signature_includes_method_path_and_body_hash --locked && cargo test -p nvbes-core derive_scope_changes_by_actor_and_route --locked` | `libs/rust/core/src/idempotency.rs` |
| events | active | `pnpm check:contracts` | `contracts/events/manifest.json` |

## Regeneration

```bash
pnpm check:migration-platform-primitives
node tools/migration/platform-primitives.mjs --write
```
