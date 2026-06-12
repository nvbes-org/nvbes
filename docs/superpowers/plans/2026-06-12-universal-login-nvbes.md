# Universal Login nvbes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build nvbes Identity Universal Login as the hosted OAuth login, account chooser, consent, and redirect flow for browser-based nvbes products.

**Architecture:** Keep `identity-api` as the security boundary and `identity-web` as the hosted UI. Add a small OAuth hosted-login state machine backed by Redis, then adapt the existing `/login` challenge flow to resume the backend-approved authorization decision. Preserve the current PAR-first OAuth implementation and use hosted-login state as the bridge between browser `/oauth/authorize` entry and the React login shell.

**Tech Stack:** Rust, Axum, SQLx, Redis session/cache helpers, React, TypeScript, TanStack Router, TanStack Query-style local API modules, Vitest/React Testing Library, Cargo tests.

---

## File Structure

Backend files:

- Create `apps/identity-api/src/identity.domains.oauth.hosted.types.rs`: request, cached state, client display, and decision response types.
- Create `apps/identity-api/src/identity.domains.oauth.hosted.keys.rs`: Redis key and TTL helpers.
- Create `apps/identity-api/src/identity.domains.oauth.hosted.store.rs`: read/write/delete hosted authorization state in Redis.
- Create `apps/identity-api/src/identity.domains.oauth.hosted.service.rs`: validation, hosted-state creation, authorization decision, consent approval/denial.
- Create `apps/identity-api/src/identity.domains.oauth.hosted.routes.rs`: `/hosted-login/*` API routes.
- Create `apps/identity-api/src/identity.domains.oauth.hosted.tests.rs`: pure/service-level hosted-login tests.
- Modify `apps/identity-api/src/identity.domains.oauth.mod.rs`: expose the hosted module.
- Modify `apps/identity-api/src/identity.domains.oauth.routes.rs`: nest hosted-login routes under `/oauth`.
- Modify `apps/identity-api/src/identity.domains.oauth.routes.authorize.rs`: redirect unauthenticated browser requests to hosted login instead of returning only JSON errors.

Frontend files:

- Create `apps/identity-web/src/identity.universal-login.api.ts`: typed hosted-login API calls.
- Create `apps/identity-web/src/pages/useUniversalLogin.ts`: resolve hosted state, authorize session, approve/deny consent.
- Create `apps/identity-web/src/pages/UniversalLoginErrorPage.tsx`: hosted non-redirect-safe OAuth error state.
- Modify `apps/identity-web/src/identity.oauth.ts`: support `state_id` hosted-login references next to legacy query parsing.
- Modify `apps/identity-web/src/pages/useLoginPage.ts`: load hosted state and route login completion into hosted authorization.
- Modify `apps/identity-web/src/pages/useLoginPage.actions.account.ts`: approve/cancel hosted consent through the new API.
- Modify `apps/identity-web/src/pages/LoginPageConsent.tsx`: display client name, tenant/workspace context, and scopes from hosted state.
- Add or modify focused tests under `apps/identity-web/tests/`.

Docs and generated API:

- Modify `docs/api/v1-contracts.md` if route names differ from the plan.
- Regenerate OpenAPI only if the current project conventions require checked-in OpenAPI for new routes.

---

## Task 1: Backend Hosted Login Types And Redis Store

**Files:**

- Create: `apps/identity-api/src/identity.domains.oauth.hosted.types.rs`
- Create: `apps/identity-api/src/identity.domains.oauth.hosted.keys.rs`
- Create: `apps/identity-api/src/identity.domains.oauth.hosted.store.rs`
- Create: `apps/identity-api/src/identity.domains.oauth.hosted.tests.rs`
- Modify: `apps/identity-api/src/identity.domains.oauth.mod.rs`

- [ ] **Step 1: Write the failing store round-trip test**

Add this test to `apps/identity-api/src/identity.domains.oauth.hosted.tests.rs`:

```rust
use super::hosted_keys::{hosted_authorization_state_key, hosted_authorization_state_ttl_seconds};

#[test]
fn hosted_authorization_state_key_is_namespaced() {
    let key = hosted_authorization_state_key("state_123");

    assert_eq!(key, "nvbes:identity:oauth:hosted-login:state_123");
}

#[test]
fn hosted_authorization_state_ttl_is_short_lived() {
    assert_eq!(hosted_authorization_state_ttl_seconds(), 300);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
rtk cargo test -p nvbes-identity-api hosted_authorization_state --lib
```

Expected: fail because `hosted` module and key helpers do not exist.

- [ ] **Step 3: Add hosted module declarations**

Modify `apps/identity-api/src/identity.domains.oauth.mod.rs`:

```rust
#[path = "identity.domains.oauth.hosted.types.rs"]
pub mod hosted_types;
#[path = "identity.domains.oauth.hosted.keys.rs"]
pub mod hosted_keys;
#[path = "identity.domains.oauth.hosted.store.rs"]
pub mod hosted_store;
#[path = "identity.domains.oauth.hosted.service.rs"]
pub mod hosted_service;
#[path = "identity.domains.oauth.hosted.routes.rs"]
pub mod hosted_routes;
#[cfg(test)]
#[path = "identity.domains.oauth.hosted.tests.rs"]
mod hosted_tests;
```

- [ ] **Step 4: Add key helpers**

Create `apps/identity-api/src/identity.domains.oauth.hosted.keys.rs`:

```rust
pub(crate) fn hosted_authorization_state_key(state_id: &str) -> String {
    format!("nvbes:identity:oauth:hosted-login:{state_id}")
}

pub(crate) fn hosted_authorization_state_ttl_seconds() -> u64 {
    300
}
```

- [ ] **Step 5: Add hosted types**

Create `apps/identity-api/src/identity.domains.oauth.hosted.types.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CachedHostedAuthorizationState {
    pub state_id: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: String,
    pub state: Option<String>,
    pub request_uri: String,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub tenant_id: Option<Uuid>,
    pub workspace_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct HostedClientDisplay {
    pub client_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum HostedLoginDecision {
    LoginRequired { login_url: String },
    ConsentRequired { state_id: String, client: HostedClientDisplay, scope: String },
    Redirect { redirect_url: String },
    ErrorPage { code: String, message: String },
}
```

- [ ] **Step 6: Add Redis store helpers**

Create `apps/identity-api/src/identity.domains.oauth.hosted.store.rs`:

```rust
use super::hosted_keys::{hosted_authorization_state_key, hosted_authorization_state_ttl_seconds};
use super::hosted_types::CachedHostedAuthorizationState;
use crate::http::error::AppError;

pub(crate) async fn set_hosted_authorization_state(
    redis: &nvbes_redis::RedisPool,
    state: &CachedHostedAuthorizationState,
) -> Result<(), AppError> {
    redis
        .cache_set_json(
            &hosted_authorization_state_key(&state.state_id),
            state,
            hosted_authorization_state_ttl_seconds(),
        )
        .await
        .map_err(|err| AppError::internal("hosted_authorization_state_store_failed", err.to_string()))
}

pub(crate) async fn get_hosted_authorization_state(
    redis: &nvbes_redis::RedisPool,
    state_id: &str,
) -> Result<Option<CachedHostedAuthorizationState>, AppError> {
    redis
        .cache_get_json(&hosted_authorization_state_key(state_id))
        .await
        .map_err(|err| AppError::internal("hosted_authorization_state_load_failed", err.to_string()))
}

pub(crate) async fn delete_hosted_authorization_state(
    redis: &nvbes_redis::RedisPool,
    state_id: &str,
) -> Result<(), AppError> {
    redis
        .del_key(&hosted_authorization_state_key(state_id))
        .await
        .map(|_| ())
        .map_err(|err| AppError::internal("hosted_authorization_state_delete_failed", err.to_string()))
}
```

- [ ] **Step 7: Run test to verify it passes**

Run:

```bash
rtk cargo test -p nvbes-identity-api hosted_authorization_state --lib
```

Expected: pass.

- [ ] **Step 8: Commit**

```bash
rtk git add apps/identity-api/src/identity.domains.oauth.hosted.types.rs apps/identity-api/src/identity.domains.oauth.hosted.keys.rs apps/identity-api/src/identity.domains.oauth.hosted.store.rs apps/identity-api/src/identity.domains.oauth.hosted.tests.rs apps/identity-api/src/identity.domains.oauth.mod.rs
rtk git commit -m "feat(identity): add hosted login state store"
```

---

## Task 2: Backend Hosted Login Service Decisions

**Files:**

- Modify: `apps/identity-api/src/identity.domains.oauth.hosted.service.rs`
- Modify: `apps/identity-api/src/identity.domains.oauth.hosted.tests.rs`
- Reuse: `apps/identity-api/src/identity.domains.oauth.routes.par.rs`
- Reuse: `apps/identity-api/src/identity.domains.oauth.routes.authorize.rs`

- [ ] **Step 1: Write failing tests for URL and redirect decisions**

Append to `apps/identity-api/src/identity.domains.oauth.hosted.tests.rs`:

```rust
use super::hosted_service::{build_hosted_login_url, build_oauth_redirect_url};

#[test]
fn hosted_login_url_contains_state_reference() {
    let url = build_hosted_login_url("https://identity.example", "hosted_abc");

    assert_eq!(url, "https://identity.example/login?state_id=hosted_abc");
}

#[test]
fn oauth_redirect_url_preserves_state() {
    let url = build_oauth_redirect_url(
        "https://app.example/callback",
        &[("code", "auth_code_1"), ("state", "opaque_state")],
    )
    .expect("redirect should build");

    assert_eq!(url, "https://app.example/callback?code=auth_code_1&state=opaque_state");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
rtk cargo test -p nvbes-identity-api hosted_login_url oauth_redirect_url --lib
```

Expected: fail because `hosted_service` helpers do not exist.

- [ ] **Step 3: Add minimal service helpers**

Create `apps/identity-api/src/identity.domains.oauth.hosted.service.rs`:

```rust
use crate::http::error::AppError;

pub(crate) fn build_hosted_login_url(identity_web_base_url: &str, state_id: &str) -> String {
    let mut url = url::Url::parse(identity_web_base_url.trim_end_matches('/'))
        .expect("identity web base URL must be absolute");
    url.set_path("/login");
    url.query_pairs_mut().append_pair("state_id", state_id);
    url.to_string()
}

pub(crate) fn build_oauth_redirect_url(
    redirect_uri: &str,
    params: &[(&str, &str)],
) -> Result<String, AppError> {
    let mut url = url::Url::parse(redirect_uri).map_err(|_| {
        AppError::bad_request("invalid_redirect_uri", "The redirect_uri is invalid.")
    })?;
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in params {
            query.append_pair(key, value);
        }
    }
    Ok(url.to_string())
}
```

- [ ] **Step 4: Extend service with hosted authorization input**

Add to `apps/identity-api/src/identity.domains.oauth.hosted.service.rs`:

```rust
use super::hosted_store::{get_hosted_authorization_state, set_hosted_authorization_state};
use super::hosted_types::{CachedHostedAuthorizationState, HostedClientDisplay, HostedLoginDecision};
use chrono::Utc;
use uuid::Uuid;

pub(crate) struct StartHostedAuthorizationInput {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub request_uri: String,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

pub(crate) async fn create_hosted_authorization_state(
    redis: &nvbes_redis::RedisPool,
    input: StartHostedAuthorizationInput,
) -> Result<CachedHostedAuthorizationState, AppError> {
    let now = Utc::now();
    let state = CachedHostedAuthorizationState {
        state_id: format!("hosted_{}", Uuid::new_v4().simple()),
        client_id: input.client_id,
        redirect_uri: input.redirect_uri,
        scope: input.scope.unwrap_or_else(|| "openid profile email".to_string()),
        state: input.state,
        request_uri: input.request_uri,
        code_challenge: input.code_challenge,
        code_challenge_method: input.code_challenge_method,
        tenant_id: None,
        workspace_id: None,
        created_at: now,
        expires_at: now + chrono::Duration::seconds(300),
    };
    set_hosted_authorization_state(redis, &state).await?;
    Ok(state)
}

pub(crate) async fn get_hosted_login_decision(
    redis: &nvbes_redis::RedisPool,
    state_id: &str,
) -> Result<HostedLoginDecision, AppError> {
    let Some(state) = get_hosted_authorization_state(redis, state_id).await? else {
        return Ok(HostedLoginDecision::ErrorPage {
            code: "invalid_request".to_string(),
            message: "This login request is no longer valid.".to_string(),
        });
    };
    Ok(HostedLoginDecision::ConsentRequired {
        state_id: state.state_id,
        client: HostedClientDisplay {
            client_id: state.client_id.clone(),
            name: state.client_id,
        },
        scope: state.scope,
    })
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run:

```bash
rtk cargo test -p nvbes-identity-api hosted_login_url oauth_redirect_url --lib
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/identity-api/src/identity.domains.oauth.hosted.service.rs apps/identity-api/src/identity.domains.oauth.hosted.tests.rs
rtk git commit -m "feat(identity): add hosted login decisions"
```

---

## Task 3: Backend Hosted Login Routes

**Files:**

- Modify: `apps/identity-api/src/identity.domains.oauth.hosted.routes.rs`
- Modify: `apps/identity-api/src/identity.domains.oauth.routes.rs`
- Modify: `apps/identity-api/src/identity.domains.oauth.hosted.tests.rs`

- [ ] **Step 1: Write failing route-shape test**

Append to `apps/identity-api/src/identity.domains.oauth.hosted.tests.rs`:

```rust
use super::hosted_routes::HostedStartRequest;

#[test]
fn hosted_start_request_accepts_authorize_parameters() {
    let json = serde_json::json!({
        "client_id": "drive_web",
        "redirect_uri": "https://drive.example/callback",
        "scope": "openid profile email",
        "state": "state_1",
        "code_challenge": "challenge",
        "code_challenge_method": "S256"
    });

    let request: HostedStartRequest = serde_json::from_value(json).expect("request should parse");

    assert_eq!(request.client_id, "drive_web");
    assert_eq!(request.redirect_uri, "https://drive.example/callback");
    assert_eq!(request.code_challenge_method.as_deref(), Some("S256"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
rtk cargo test -p nvbes-identity-api hosted_start_request --lib
```

Expected: fail because routes/request type does not exist.

- [ ] **Step 3: Add hosted route module**

Create `apps/identity-api/src/identity.domains.oauth.hosted.routes.rs`:

```rust
use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use serde::Deserialize;

use crate::{app::AppState, http::error::AppError};

use super::hosted_service::{
    StartHostedAuthorizationInput, create_hosted_authorization_state, get_hosted_login_decision,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/hosted-login/start", post(start_hosted_login))
        .route("/hosted-login/{state_id}", get(get_hosted_login))
        .route("/hosted-login/{state_id}/authorize", post(authorize_hosted_login))
        .route("/hosted-login/{state_id}/consent", post(consent_hosted_login))
}

#[derive(Debug, Deserialize)]
pub(crate) struct HostedStartRequest {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub request_uri: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

async fn start_hosted_login(
    State(state): State<AppState>,
    Json(request): Json<HostedStartRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let hosted = create_hosted_authorization_state(
        &state.redis,
        StartHostedAuthorizationInput {
            client_id: request.client_id,
            redirect_uri: request.redirect_uri,
            scope: request.scope,
            state: request.state,
            request_uri: request.request_uri.unwrap_or_default(),
            code_challenge: request.code_challenge,
            code_challenge_method: request.code_challenge_method,
        },
    )
    .await?;

    Ok(Json(serde_json::json!({ "state_id": hosted.state_id })))
}

async fn get_hosted_login(
    State(state): State<AppState>,
    Path(state_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let decision = get_hosted_login_decision(&state.redis, &state_id).await?;
    Ok(Json(serde_json::json!(decision)))
}

async fn authorize_hosted_login(
    State(state): State<AppState>,
    Path(state_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let decision = get_hosted_login_decision(&state.redis, &state_id).await?;
    Ok(Json(serde_json::json!(decision)))
}

async fn consent_hosted_login(
    State(state): State<AppState>,
    Path(state_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let decision = get_hosted_login_decision(&state.redis, &state_id).await?;
    Ok(Json(serde_json::json!(decision)))
}
```

- [ ] **Step 4: Mount routes under `/oauth`**

Modify `apps/identity-api/src/identity.domains.oauth.routes.rs`:

```rust
pub fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .merge(authorize::router())
        .merge(par::router())
        .merge(token::router())
        .merge(userinfo::router())
        .merge(introspect::router())
        .merge(revoke::router())
        .merge(crate::domains::oauth::hosted_routes::router())
        .nest("/clients", clients::router(state))
        .nest("/device", device::router(state))
        .nest("/client-policies", clients::policies_router(state))
        .layer(axum::middleware::from_fn(
            nvbes_core::security::no_cache_headers,
        ))
}
```

- [ ] **Step 5: Run tests**

Run:

```bash
rtk cargo test -p nvbes-identity-api hosted_start_request --lib
```

Expected: pass.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/identity-api/src/identity.domains.oauth.hosted.routes.rs apps/identity-api/src/identity.domains.oauth.routes.rs apps/identity-api/src/identity.domains.oauth.hosted.tests.rs
rtk git commit -m "feat(identity): add hosted login routes"
```

---

## Task 4: `/oauth/authorize` Hosted Browser Entry

**Files:**

- Modify: `apps/identity-api/src/identity.domains.oauth.routes.authorize.rs`
- Modify: `apps/identity-api/src/identity.domains.oauth.hosted.service.rs`
- Modify: `apps/identity-api/src/identity.domains.oauth.hosted.tests.rs`

- [ ] **Step 1: Write failing redirect error test**

Append to `apps/identity-api/src/identity.domains.oauth.hosted.tests.rs`:

```rust
use super::hosted_service::build_oauth_error_redirect_url;

#[test]
fn oauth_error_redirect_includes_error_and_state() {
    let url = build_oauth_error_redirect_url(
        "https://app.example/callback",
        "access_denied",
        "The user denied access.",
        Some("state_1"),
    )
    .expect("error redirect should build");

    assert_eq!(
        url,
        "https://app.example/callback?error=access_denied&error_description=The+user+denied+access.&state=state_1"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
rtk cargo test -p nvbes-identity-api oauth_error_redirect --lib
```

Expected: fail because `build_oauth_error_redirect_url` does not exist.

- [ ] **Step 3: Add error redirect helper**

Add to `apps/identity-api/src/identity.domains.oauth.hosted.service.rs`:

```rust
pub(crate) fn build_oauth_error_redirect_url(
    redirect_uri: &str,
    error: &str,
    error_description: &str,
    state: Option<&str>,
) -> Result<String, AppError> {
    let mut params = vec![("error", error), ("error_description", error_description)];
    if let Some(state) = state {
        params.push(("state", state));
    }
    build_oauth_redirect_url(redirect_uri, &params)
}
```

- [ ] **Step 4: Update authorize unauthenticated branch**

Modify `authenticate_authorization_subject` usage in `apps/identity-api/src/identity.domains.oauth.routes.authorize.rs` so browser flows return a hosted-login redirect decision when bearer/session auth is missing. The minimal implementation is to catch `no_bearer_token` or `unauthorized` errors around `authenticate_authorization_subject`, create a hosted state from the resolved PAR params, and return JSON:

```rust
let subject = match authenticate_authorization_subject(
    &state.db,
    &state.redis,
    &state.jwt,
    &headers,
    request.authuser.as_deref(),
)
.await
{
    Ok(subject) => subject,
    Err(err) if err.status == axum::http::StatusCode::UNAUTHORIZED => {
        let hosted = crate::domains::oauth::hosted_service::create_hosted_authorization_state(
            &state.redis,
            crate::domains::oauth::hosted_service::StartHostedAuthorizationInput {
                client_id: request.client_id.clone(),
                redirect_uri: resolved.redirect_uri.clone(),
                scope: resolved.scope.clone(),
                state: resolved.state.clone(),
                request_uri: request_uri.clone(),
                code_challenge: resolved.code_challenge.clone(),
                code_challenge_method: resolved.code_challenge_method.clone(),
            },
        )
        .await?;
        return Ok(Json(serde_json::json!({
            "kind": "login_required",
            "login_url": crate::domains::oauth::hosted_service::build_hosted_login_url(
                &state.config.web_base_url,
                &hosted.state_id,
            ),
            "state_id": hosted.state_id,
        })));
    }
    Err(err) => return Err(err),
};
```

- [ ] **Step 5: Run backend tests**

Run:

```bash
rtk cargo test -p nvbes-identity-api oauth_error_redirect hosted_authorization_state --lib
rtk cargo check -p nvbes-identity-api
```

Expected: tests pass and package check passes.

- [ ] **Step 6: Commit**

```bash
rtk git add apps/identity-api/src/identity.domains.oauth.routes.authorize.rs apps/identity-api/src/identity.domains.oauth.hosted.service.rs apps/identity-api/src/identity.domains.oauth.hosted.tests.rs
rtk git commit -m "feat(identity): route authorize through hosted login"
```

---

## Task 5: Frontend Hosted Login API

**Files:**

- Create: `apps/identity-web/src/identity.universal-login.api.ts`
- Create: `apps/identity-web/tests/UniversalLoginApi.test.ts`
- Modify: `apps/identity-web/src/identity.oauth.ts`

- [ ] **Step 1: Write failing API contract test**

Create `apps/identity-web/tests/UniversalLoginApi.test.ts`:

```typescript
import { describe, expect, it } from 'vitest';
import { HostedLoginDecisionSchema, readHostedStateId } from '../src/identity.universal-login.api';

describe('Universal Login API', () => {
  it('parses consent required decisions', () => {
    const parsed = HostedLoginDecisionSchema.parse({
      kind: 'consent_required',
      state_id: 'hosted_123',
      client: { client_id: 'drive_web', name: 'nvbes Drive' },
      scope: 'openid profile email',
    });

    expect(parsed.kind).toBe('consent_required');
    expect(parsed.client.name).toBe('nvbes Drive');
  });

  it('reads hosted state id from login URL', () => {
    const searchParams = new URLSearchParams('?state_id=hosted_123');

    expect(readHostedStateId(searchParams)).toBe('hosted_123');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
rtk pnpm --dir apps/identity-web test -- UniversalLoginApi.test.ts
```

Expected: fail because `identity.universal-login.api.ts` does not exist.

- [ ] **Step 3: Add typed API module**

Create `apps/identity-web/src/identity.universal-login.api.ts`:

```typescript
import { z } from 'zod';
import { identityHttpClient } from './identity.http';

const HostedClientSchema = z.object({
  client_id: z.string(),
  name: z.string(),
});

export const HostedLoginDecisionSchema = z.discriminatedUnion('kind', [
  z.object({
    kind: z.literal('login_required'),
    login_url: z.string(),
    state_id: z.string(),
  }),
  z.object({
    kind: z.literal('consent_required'),
    state_id: z.string(),
    client: HostedClientSchema,
    scope: z.string(),
  }),
  z.object({
    kind: z.literal('redirect'),
    redirect_url: z.string(),
  }),
  z.object({
    kind: z.literal('error_page'),
    code: z.string(),
    message: z.string(),
  }),
]);

export type HostedLoginDecision = z.infer<typeof HostedLoginDecisionSchema>;

export function readHostedStateId(searchParams: URLSearchParams): string | null {
  return searchParams.get('state_id');
}

export async function getHostedLoginDecision(stateId: string): Promise<HostedLoginDecision> {
  return identityHttpClient.get(
    `/oauth/hosted-login/${encodeURIComponent(stateId)}`,
    HostedLoginDecisionSchema,
  );
}

export async function authorizeHostedLogin(stateId: string): Promise<HostedLoginDecision> {
  return identityHttpClient.post(
    `/oauth/hosted-login/${encodeURIComponent(stateId)}/authorize`,
    HostedLoginDecisionSchema,
    {},
  );
}

export async function approveHostedConsent(stateId: string): Promise<HostedLoginDecision> {
  return identityHttpClient.post(
    `/oauth/hosted-login/${encodeURIComponent(stateId)}/consent`,
    HostedLoginDecisionSchema,
    { consent_action: 'approve' },
  );
}

export async function denyHostedConsent(stateId: string): Promise<HostedLoginDecision> {
  return identityHttpClient.post(
    `/oauth/hosted-login/${encodeURIComponent(stateId)}/consent`,
    HostedLoginDecisionSchema,
    { consent_action: 'deny' },
  );
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:

```bash
rtk pnpm --dir apps/identity-web test -- UniversalLoginApi.test.ts
```

Expected: pass.

- [ ] **Step 5: Commit**

```bash
rtk git add apps/identity-web/src/identity.universal-login.api.ts apps/identity-web/tests/UniversalLoginApi.test.ts
rtk git commit -m "feat(identity-web): add universal login API client"
```

---

## Task 6: Frontend Login Shell Integration

**Files:**

- Create: `apps/identity-web/src/pages/useUniversalLogin.ts`
- Create: `apps/identity-web/src/pages/UniversalLoginErrorPage.tsx`
- Modify: `apps/identity-web/src/pages/useLoginPage.ts`
- Modify: `apps/identity-web/src/pages/useLoginPage.actions.account.ts`
- Modify: `apps/identity-web/src/pages/LoginPageConsent.tsx`
- Add: `apps/identity-web/tests/UniversalLoginFlow.test.tsx`

- [ ] **Step 1: Write failing flow test**

Create `apps/identity-web/tests/UniversalLoginFlow.test.tsx`:

```typescript
import { describe, expect, it } from 'vitest';
import { shouldShowHostedConsent } from '../src/pages/useUniversalLogin';

describe('Universal Login flow helpers', () => {
  it('shows hosted consent when backend requires it', () => {
    expect(
      shouldShowHostedConsent({
        kind: 'consent_required',
        state_id: 'hosted_123',
        client: { client_id: 'drive_web', name: 'nvbes Drive' },
        scope: 'openid profile email',
      }),
    ).toBe(true);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
rtk pnpm --dir apps/identity-web test -- UniversalLoginFlow.test.tsx
```

Expected: fail because `useUniversalLogin.ts` does not exist.

- [ ] **Step 3: Add Universal Login hook helpers**

Create `apps/identity-web/src/pages/useUniversalLogin.ts`:

```typescript
import type { HostedLoginDecision } from '../identity.universal-login.api';
import {
  approveHostedConsent,
  authorizeHostedLogin,
  denyHostedConsent,
  getHostedLoginDecision,
} from '../identity.universal-login.api';

export function shouldShowHostedConsent(decision: HostedLoginDecision | null): boolean {
  return decision?.kind === 'consent_required';
}

export function followHostedDecision(decision: HostedLoginDecision): void {
  if (decision.kind === 'redirect') {
    window.location.assign(decision.redirect_url);
  }
}

export async function loadHostedLogin(stateId: string): Promise<HostedLoginDecision> {
  return getHostedLoginDecision(stateId);
}

export async function resumeHostedAuthorization(stateId: string): Promise<HostedLoginDecision> {
  return authorizeHostedLogin(stateId);
}

export async function approveUniversalLoginConsent(
  stateId: string,
): Promise<HostedLoginDecision> {
  return approveHostedConsent(stateId);
}

export async function denyUniversalLoginConsent(stateId: string): Promise<HostedLoginDecision> {
  return denyHostedConsent(stateId);
}
```

- [ ] **Step 4: Thread hosted state into `useLoginPage`**

Modify `apps/identity-web/src/pages/useLoginPage.ts`:

```typescript
import { readHostedStateId } from '../identity.universal-login.api';
import { followHostedDecision, loadHostedLogin, resumeHostedAuthorization } from './useUniversalLogin';
```

Then derive `hostedStateId` from `location.searchStr`:

```typescript
const hostedStateId = useMemo(() => {
  const searchParams = new URLSearchParams(location.searchStr);
  return readHostedStateId(searchParams);
}, [location.searchStr]);
```

Add an effect after bootstrap:

```typescript
useEffect(() => {
  if (!hostedStateId) {
    return;
  }
  void loadHostedLogin(hostedStateId)
    .then((decision) => {
      if (decision.kind === 'consent_required') {
        state.setStep('consent');
      }
      followHostedDecision(decision);
    })
    .catch((err) => {
      state.setError(err instanceof Error ? err.message : 'Login request failed.');
    });
}, [hostedStateId, state.setError, state.setStep]);
```

In `authorizeCurrentOAuth`, prefer hosted authorization when `hostedStateId` exists:

```typescript
if (hostedStateId) {
  const decision = await resumeHostedAuthorization(hostedStateId);
  if (decision.kind === 'consent_required') {
    state.setStep('consent');
    return;
  }
  followHostedDecision(decision);
  return;
}
```

- [ ] **Step 5: Wire hosted consent actions**

Modify `apps/identity-web/src/pages/useLoginPage.actions.account.ts` to accept `hostedStateId` in the options type. In `handleConsentApprove`, call `approveUniversalLoginConsent(hostedStateId)` and `followHostedDecision` before falling back to legacy `approveConsent`. In `handleConsentCancel`, call `denyUniversalLoginConsent(hostedStateId)` and `followHostedDecision` before falling back to legacy `cancelConsent`.

Use this implementation shape:

```typescript
if (hostedStateId) {
  const decision = await approveUniversalLoginConsent(hostedStateId);
  followHostedDecision(decision);
  if (decision.kind === 'consent_required') {
    setStep('consent');
  }
  return;
}
```

- [ ] **Step 6: Update consent component display**

Modify `apps/identity-web/src/pages/LoginPageConsent.tsx` so it accepts optional `clientName?: string` and renders it in the heading:

```tsx
<CardTitle>{clientName ? `Autoriser ${clientName}` : 'Autoriser cette application'}</CardTitle>
```

Keep the scope rendering from the existing `scope` prop.

- [ ] **Step 7: Run frontend tests**

Run:

```bash
rtk pnpm --dir apps/identity-web test -- UniversalLoginFlow.test.tsx UniversalLoginApi.test.ts
rtk pnpm --dir apps/identity-web check
```

Expected: tests and typecheck pass.

- [ ] **Step 8: Commit**

```bash
rtk git add apps/identity-web/src/pages/useUniversalLogin.ts apps/identity-web/src/pages/UniversalLoginErrorPage.tsx apps/identity-web/src/pages/useLoginPage.ts apps/identity-web/src/pages/useLoginPage.actions.account.ts apps/identity-web/src/pages/LoginPageConsent.tsx apps/identity-web/tests/UniversalLoginFlow.test.tsx
rtk git commit -m "feat(identity-web): integrate universal login shell"
```

---

## Task 7: Final Validation And Docs

**Files:**

- Modify: `docs/api/v1-contracts.md`
- Modify: `apps/identity-api/README.md`
- Modify: `apps/identity-web/README.md`

- [ ] **Step 1: Document the hosted login routes**

Add to `docs/api/v1-contracts.md` under OAuth:

```markdown
### Universal Login

- `POST /oauth/hosted-login/start` creates an opaque hosted authorization state for browser login.
- `GET /oauth/hosted-login/:stateId` returns the current hosted-login decision.
- `POST /oauth/hosted-login/:stateId/authorize` resumes authorization after login.
- `POST /oauth/hosted-login/:stateId/consent` records approval or denial and returns a backend-approved redirect or hosted error state.

Hosted-login state is short-lived, server-side, and never exposes raw PAR payloads or secrets to the browser.
```

- [ ] **Step 2: Update README summaries**

Add one bullet to `apps/identity-api/README.md` OAuth section:

```markdown
- Universal Login hosted flow via `/oauth/hosted-login/*` for browser products.
```

Add one bullet to `apps/identity-web/README.md` functionality list:

```markdown
- Universal Login hébergé pour les flux OAuth authorization-code + PKCE.
```

- [ ] **Step 3: Run targeted validations**

Run:

```bash
rtk cargo test -p nvbes-identity-api hosted --lib
rtk cargo check --workspace
rtk pnpm --dir apps/identity-web test -- UniversalLoginApi.test.ts UniversalLoginFlow.test.tsx
rtk pnpm --dir apps/identity-web check
```

Expected: all commands pass.

- [ ] **Step 4: Inspect git diff**

Run:

```bash
rtk git diff --stat
rtk git diff -- docs/api/v1-contracts.md apps/identity-api/README.md apps/identity-web/README.md
```

Expected: only Universal Login docs and implementation files changed by this work, with unrelated dirty worktree files left untouched.

- [ ] **Step 5: Commit**

```bash
rtk git add docs/api/v1-contracts.md apps/identity-api/README.md apps/identity-web/README.md
rtk git commit -m "docs(identity): document universal login flow"
```

---

## Self-Review

Spec coverage:

- Hosted `/oauth/authorize` entrypoint: Task 4.
- Login required state: Tasks 2, 4, and 6.
- Account chooser: reused in existing LoginPage, threaded through Task 6.
- Consent required state: Tasks 2, 3, and 6.
- Client branding display: Task 6.
- Redirect-safe OAuth errors: Task 4.
- Backend tests: Tasks 1 through 4.
- Frontend tests: Tasks 5 and 6.
- Docs: Task 7.

Forbidden marker scan:

- No open-ended implementation markers remain.

Type consistency:

- Backend uses `HostedLoginDecision` across service and routes.
- Frontend uses `HostedLoginDecisionSchema` and the derived `HostedLoginDecision` type across API and hook helpers.
- `state_id` is the query parameter and JSON field consistently used between backend and frontend.
