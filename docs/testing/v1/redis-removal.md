# V1 without Redis

## Decision and scope

Redis is removed from the active Cargo workspaces, runtime configuration,
test runner, local Compose stacks and CI provisioning. Historical source remains
versioned under `archive/libs/rust/redis/`; archived code is not reactivated or
rewritten.

Existing runtime sources of truth remain unchanged:

- Account exports and closure work: PostgreSQL `account_exports` and jobs.
- Billing webhook processing and checkout idempotency: PostgreSQL transactions
  and Stripe idempotency keys, in the active Billing service.
- Identity recovery delivery: Email service client, not the old Redis jobs.
- Email: PostgreSQL persistence and the existing dispatch implementation;
  external delivery remains at-least-once.

Unused Redis queue/cache APIs are removed, not silently redirected to a different
delivery contract. The pure Account export builder and idempotency helpers remain.

## Security library contracts

`nvbes_core::limiter::RateLimiter` now takes `PgPool`.
`nvbes_dpop::DpopNonceStore::new(PgPool, ttl)` now returns `Result` and rejects
lifetimes outside 1–86400 seconds. Callers must handle this API change.

Before using these libraries, the owning application's migration role must apply
`nvbes_core::limiter::SCHEMA` and/or `nvbes_dpop::nonce::SCHEMA` in its own database.
Runtime pools need only SELECT, INSERT, UPDATE and DELETE on the corresponding
tables. The libraries never create schemas or tables during a request.

Nonce consumption is single-use and checks expiration after any row-lock wait.
JTI claims are atomic and scoped to the proof key. Rate-limit increments are atomic,
with a single observation time for a reset. Storage failure returns an error;
there is no in-memory or permissive fallback. Keys are hashed at rest.

The owning maintenance loop must call `cleanup_expired`; each call removes at
most 1000 expired rows, skipping locked rows. PostgreSQL availability and cleanup
capacity must be included in the caller's capacity test.

These primitives were not wired into the inspected active HTTP services before
this change. Replacing their backend does not claim that DPoP or rate limiting is
now enforced on additional endpoints. Endpoint integration and policy evidence
remain separate V1 release obligations; no security requirement is excluded.

## Validation

Security library tests require `NVBES_SECURITY_TEST_DATABASE_URL` pointing to an
ephemeral local database ending in `_test`. Missing/unreachable PostgreSQL fails
the tests. Each test gets its own schema. The harness must remove the disposable
database after execution. In CI the exact job-scoped PostgreSQL hostname is also
accepted; arbitrary remote database hosts remain forbidden.

Run workspace checks, Clippy, and the core/DPoP/product/test-utils library tests.
Run `node --test tools/ci/redis-removal.test.mjs` for dependency and CI contracts.
Coverage, branches and mutation campaigns also require the security test database;
previous-SHA measurements are not evidence for this change. V1 thresholds are not
lowered. Legacy security registries referencing archived routes are not a GO proof.

## Local transition

Rebuild/recreate the development stack from the new Compose definitions when no
other task uses it. Remove obsolete `NVBES_REDIS_*`/`REDIS_PASSWORD` variables from
private environments after a private backup. Do not destroy Redis volumes while
an older checkout or a running CI job still uses them. This change does not stop
existing containers, erase their data, alter deployed secrets or migrate live state.

If any deployed caller still uses the removed APIs, drain its jobs and expire or
migrate its security state before rollout. Never reset replay state while proofs
using that state are still valid. Rollback must use the previous matching code and
configuration; it is not safe to switch security stores under live traffic.
