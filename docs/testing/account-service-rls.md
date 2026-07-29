# Account Service RLS: reference contract and production gaps

## Production status

The Account Service does **not** currently enforce the hardened RLS reference
contract in production. Promoting that contract to a migration would break
valid request and background paths because most handlers still execute through
a shared `PgPool`, without a transaction-scoped tenant context.

The executable baseline audit records the current gaps:

- `devices`, `saml_assertion_ids`, `saml_pending_requests`,
  `saml_sp_config`, `security_event_deliveries` and `user_sessions` have a
  `tenant_id` column but no RLS;
- established tenant tables such as `tenants`, `oauth_clients` and
  `workspaces` enable RLS without `FORCE ROW LEVEL SECURITY`;
- `system_policy_read` is a `FOR ALL` policy despite its name;
- the shared role-scoped pool initializes RLS identifiers with the nil UUID,
  which collides with the seeded nil-UUID system tenant;
- most Account handlers use `PgPool` directly; the password-review path is a
  rare path that establishes transaction-scoped RLS context;
- bootstrap seeding, CORS refresh and the security-event dispatcher perform
  cross-tenant or global work through the same application database model.

The audit intentionally fails when this inventory changes. A passing test means
the gaps remain accurately documented, not that production RLS is complete.

## Test-only hardened reference

[`identity.database.rls.reference.sql`](../../apps/account-service/src/identity.database.rls.reference.sql)
is deliberately outside the migration directory. It is installed only in a
uniquely named test database and specifies the target isolation behavior:

- every tenant-scoped table has a policy;
- every RLS table uses `FORCE ROW LEVEL SECURITY`;
- the six uncovered tables receive `FOR ALL` tenant policies;
- `user_sessions` requires both tenant and principal context;
- an absent context denies reads, including nil-UUID system rows;
- `system_policy_read` permits `SELECT` only.

The reference test provisions a unique login role with `NOSUPERUSER`,
`NOBYPASSRLS`, `NOCREATEDB`, `NOCREATEROLE`, `NOINHERIT` and
`NOREPLICATION`. It owns neither the database nor public tables and cannot
assume a privileged role. The test exercises tenant A/B reads and writes,
cross-tenant denial, commit/rollback context isolation and system-policy write
denial.

The same test also runs a real `AppState::bootstrap` against the unmodified
baseline schema. That smoke check documents the currently working
owner-privileged startup path; it does not claim that bootstrap works with the
hardened runtime role.

## Safety and prerequisites

The test creates and drops databases and roles. It rejects non-loopback
PostgreSQL URLs, maintenance databases, production-like names and names without
an exact `test` segment. The operator must also set
`NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE=account-quality-v1`.
`DATABASE_URL` (or `NVBES_DATABASE_URL`) must point to a disposable local
application database whose role can:

- connect to the `postgres` maintenance database;
- create databases and roles;
- install the extensions and schema objects required by Account migrations.

The migration chain may create the cluster-wide `nvbes_system` role. Use a
disposable PostgreSQL cluster, never a shared or production cluster. Redis must
also be available for the `AppState` bootstrap smoke.

Run the security lane through Nx:

```bash
DATABASE_URL=postgres://postgres:postgres@localhost:5432/nvbes_test \
NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE=account-quality-v1 \
NVBES_REDIS_URL=redis://localhost:6379 \
pnpm nx run account-service:test-security
```

The migration lane also selects this contract because the test name includes
both `security` and `migration`.

## Promotion gate

Before the reference SQL can become a production migration:

1. move tenant data access behind transaction-bound APIs that always establish
   principal, tenant and workspace context;
2. replace the nil-UUID pool sentinel with a fail-closed representation;
3. separate migration ownership from the runtime role;
4. give bootstrap, CORS refresh and dispatchers narrowly scoped system
   capabilities instead of a general `BYPASSRLS` path;
5. run the full Account service, worker and web integration suites against the
   hardened role.
