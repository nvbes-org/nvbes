# Enterprise Web Tenant Admin API Implementation Plan

> **Status: inactive future plan.** Enterprise administration APIs are outside
> the B2C foundation V1. See the
> [active direction](../../product/nvbes-product-strategy.md).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add tenant-first `account-service` routes required by `enterprise-web` Users V0.

**Architecture:** Create a flat `identity.domains.enterprise.*` module that authenticates current tenant context, authorizes from database state, exposes tenant read models, and writes audit events for membership changes.

**Tech Stack:** Rust stable, Axum, SQLx, PostgreSQL, existing `account-service` auth/session/authz modules.

---

## File Structure

- `apps/account-service/src/identity.domains.enterprise.mod.rs`: module declarations.
- `apps/account-service/src/identity.domains.enterprise.routes.rs`: Axum route handlers.
- `apps/account-service/src/identity.domains.enterprise.types.rs`: request and response DTOs.
- `apps/account-service/src/identity.domains.enterprise.service.rs`: orchestration.
- `apps/account-service/src/identity.domains.enterprise.db.rs`: SQLx read models and mutations.
- `apps/account-service/src/identity.domains.enterprise.policy.rs`: tenant admin authorization helpers.
- `apps/account-service/src/identity.domains.enterprise.tests.rs`: targeted tests.
- `apps/account-service/src/identity.domains.mod.rs`: router merge.

## Task 1: Add Enterprise Module Skeleton

**Files:**

- Create: `apps/account-service/src/identity.domains.enterprise.mod.rs`
- Create: `apps/account-service/src/identity.domains.enterprise.types.rs`
- Modify: `apps/account-service/src/identity.domains.mod.rs`

- [ ] **Step 1: Add module declarations**

```rust
#[path = "identity.domains.enterprise.db.rs"]
pub mod db;
#[path = "identity.domains.enterprise.policy.rs"]
pub mod policy;
#[path = "identity.domains.enterprise.routes.rs"]
pub mod routes;
#[path = "identity.domains.enterprise.service.rs"]
pub mod service;
#[path = "identity.domains.enterprise.types.rs"]
pub mod types;
```

- [ ] **Step 2: Add DTO types**

Define serde DTOs for context, overview, users, invitation input, access update input, audit reason input, workspace summaries, developer summaries, policy summaries, security summaries, audit events, billing summary, and usage summary. Use snake_case fields to match existing API conventions.

- [ ] **Step 3: Wire router merge**

In `identity.domains.mod.rs`, add:

```rust
#[path = "identity.domains.enterprise.mod.rs"]
pub mod enterprise;
```

and merge `enterprise::routes::router(state)` with the other domain routers.

- [ ] **Step 4: Verify and commit**

```bash
rtk cargo check --workspace
git add apps/account-service/src/identity.domains.enterprise.mod.rs apps/account-service/src/identity.domains.enterprise.types.rs apps/account-service/src/identity.domains.mod.rs
git commit -m "feat(account-service): add enterprise module skeleton"
```

Expected: workspace compiles.

## Task 2: Add Routes And Policy Boundary

**Files:**

- Create: `apps/account-service/src/identity.domains.enterprise.routes.rs`
- Create: `apps/account-service/src/identity.domains.enterprise.policy.rs`

- [ ] **Step 1: Add route tree**

Routes:

- `GET /enterprise/context`
- `GET /enterprise/overview`
- `GET /enterprise/users`
- `POST /enterprise/invitations`
- `PATCH /enterprise/users/{userId}/access`
- `POST /enterprise/users/{userId}/suspend`
- `POST /enterprise/users/{userId}/reactivate`
- `GET /enterprise/workspaces`
- `GET /enterprise/developers`
- `GET /enterprise/policies`
- `GET /enterprise/security`
- `GET /enterprise/audit-events`
- `GET /enterprise/billing`
- `GET /enterprise/usage`

Every handler authenticates the current user and requires `auth.tenant_id`.

- [ ] **Step 2: Add policy helpers**

```rust
pub fn can_manage_members(role: &str, grants: &[String]) -> bool {
    role == "owner" || (role == "admin" && grants.iter().any(|grant| grant == "members"))
}

pub fn is_last_owner_removal(owner_count: i64, current_role: &str, next_role: &str) -> bool {
    owner_count <= 1 && current_role == "owner" && next_role != "owner"
}
```

Route handlers must call service functions that authorize from database state, not from frontend payload role data.

- [ ] **Step 3: Verify and commit**

```bash
rtk cargo check --workspace
git add apps/account-service/src/identity.domains.enterprise.routes.rs apps/account-service/src/identity.domains.enterprise.policy.rs
git commit -m "feat(account-service): add enterprise route boundary"
```

Expected: workspace compiles.

## Task 3: Add Service And DB Logic

**Files:**

- Create: `apps/account-service/src/identity.domains.enterprise.service.rs`
- Create: `apps/account-service/src/identity.domains.enterprise.db.rs`

- [ ] **Step 1: Add service functions**

Services must take exact dependencies such as `&PgPool` and config references. Do not pass `&AppState`.

Required functions:

- `get_context`
- `get_overview`
- `list_users`
- `create_invitations`
- `update_user_access`
- `suspend_user`
- `reactivate_user`
- summary getters for workspaces, developers, policies, security, audit events, billing, and usage.

- [ ] **Step 2: Add DB functions**

DB functions must return tenant-scoped rows only. Every query that reads or writes tenant admin data includes `tenant_id = $1` or an equivalent tenant membership join.

Mutation functions must write audit events with actor, action, target, tenant, optional workspace, before/after summary, and timestamp.

- [ ] **Step 3: Verify and commit**

```bash
rtk cargo check --workspace
git add apps/account-service/src/identity.domains.enterprise.service.rs apps/account-service/src/identity.domains.enterprise.db.rs
git commit -m "feat(account-service): implement enterprise admin service"
```

Expected: workspace compiles.

## Task 4: Add Guardrail Tests

**Files:**

- Create: `apps/account-service/src/identity.domains.enterprise.tests.rs`
- Modify: `apps/account-service/src/identity.domains.enterprise.mod.rs`

- [ ] **Step 1: Wire test module**

```rust
#[cfg(test)]
#[path = "identity.domains.enterprise.tests.rs"]
mod tests;
```

- [ ] **Step 2: Add targeted tests**

Cover:

- tenant isolation;
- missing tenant membership denial;
- admin without Members grant denial for invite and edit;
- last owner cannot be suspended or downgraded;
- invitation lifecycle;
- duplicate invitation handling;
- workspace assignment authorization;
- audit event written for access change, suspension, and reactivation.

- [ ] **Step 3: Verify and commit**

```bash
rtk cargo test -p nvbes-account-service enterprise
rtk cargo check --workspace
git add apps/account-service/src/identity.domains.enterprise.*
git commit -m "test(account-service): cover enterprise admin guardrails"
```

Expected: targeted tests and workspace check pass.
