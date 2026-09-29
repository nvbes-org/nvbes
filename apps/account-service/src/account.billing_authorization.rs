//! Account owns billing-target authorization; Billing never replicates memberships.
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Request, State},
    http::StatusCode,
    middleware::{Next, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Clone)]
struct Authority {
    db: PgPool,
    secret_hash: [u8; 32],
    permits: Arc<Semaphore>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AccountType {
    Principal,
    Team,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Target {
    pub principal_id: Uuid,
    pub account_id: Uuid,
    pub account_type: AccountType,
}

#[derive(Serialize)]
struct Decision {
    #[serde(flatten)]
    target: Target,
    allowed: bool,
}

pub fn valid_secret(secret: &str) -> bool {
    secret.len() == 64
        && secret
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub fn router(db: PgPool, secret: Option<&str>) -> Router {
    let Some(secret) = secret else {
        return Router::new();
    };
    let state = Authority {
        db,
        secret_hash: Sha256::digest(secret.as_bytes()).into(),
        permits: Arc::new(Semaphore::new(16)),
    };
    Router::new()
        .route("/internal/v1/billing/authorize", post(authorize))
        .layer(DefaultBodyLimit::max(1024))
        .route_layer(from_fn_with_state(state.clone(), protect))
        .with_state(state)
}

async fn protect(State(state): State<Authority>, request: Request, next: Next) -> Response {
    let mut headers = request.headers().get_all("authorization").iter();
    let supplied = headers
        .next()
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .filter(|s| valid_secret(s));
    let authenticated = supplied.is_some_and(|s| {
        let hash: [u8; 32] = Sha256::digest(s.as_bytes()).into();
        bool::from(hash.ct_eq(&state.secret_hash))
    }) && headers.next().is_none();
    let mut response = if !authenticated {
        StatusCode::UNAUTHORIZED.into_response()
    } else if let Ok(_permit) = state.permits.try_acquire() {
        match tokio::time::timeout(Duration::from_secs(2), next.run(request)).await {
            Ok(response) => response,
            Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
        }
    } else {
        StatusCode::SERVICE_UNAVAILABLE.into_response()
    };
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
}

async fn authorize(State(state): State<Authority>, Json(target): Json<Target>) -> Response {
    if target.principal_id.is_nil() || target.account_id.is_nil() {
        return StatusCode::BAD_REQUEST.into_response();
    }
    match allowed(&state.db, &target).await {
        Ok(allowed) => Json(Decision { target, allowed }).into_response(),
        Err(_) => StatusCode::SERVICE_UNAVAILABLE.into_response(),
    }
}

pub(crate) async fn allowed(db: &PgPool, target: &Target) -> Result<bool, sqlx::Error> {
    let query = match target.account_type {
        AccountType::Principal => {
            "SELECT EXISTS(SELECT 1 FROM account_profiles WHERE principal_id=$1 AND principal_id=$2 AND lifecycle_status='active')"
        }
        AccountType::Team => {
            "SELECT EXISTS(SELECT 1 FROM account_teams t JOIN account_team_memberships m ON m.team_id=t.id JOIN account_profiles p ON p.principal_id=m.principal_id WHERE t.id=$2 AND t.status='active' AND t.owner_principal_id=$1 AND m.principal_id=$1 AND m.role='owner' AND p.lifecycle_status='active')"
        }
    };
    sqlx::query_scalar(query)
        .bind(target.principal_id)
        .bind(target.account_id)
        .fetch_one(db)
        .await
}

#[cfg(all(test, feature = "database-tests"))]
#[path = "account.billing_authorization.tests.rs"]
mod tests;
