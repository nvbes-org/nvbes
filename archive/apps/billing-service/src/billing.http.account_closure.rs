use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use chrono::Utc;
use nvbes_product_account::closure_event::{AccountClosureEventError, AccountClosureRequestedV1};
use serde::Serialize;
use sqlx::{Postgres, Transaction};

use crate::{app::BillingAppState, http::error::AppError};

#[derive(Serialize)]
struct AccountClosureResult {
    applied: bool,
}

pub fn router() -> Router<BillingAppState> {
    Router::new().route("/internal/v1/account-closures", post(close_account))
}

async fn close_account(
    State(state): State<BillingAppState>,
    headers: HeaderMap,
    Json(command): Json<AccountClosureRequestedV1>,
) -> Result<Json<AccountClosureResult>, AppError> {
    if !nvbes_core::http::internal_service::bearer_matches(&headers, &state.internal_service_token)
    {
        return Err(AppError::unauthorized(
            "invalid_internal_token",
            "A valid Billing internal token is required.",
        ));
    }
    validate_command(&command)?;
    let applied = apply(&state.db, &command).await?;
    Ok(Json(AccountClosureResult { applied }))
}

fn validate_command(command: &AccountClosureRequestedV1) -> Result<(), AppError> {
    match command.validate(Utc::now()) {
        Ok(()) => Ok(()),
        Err(AccountClosureEventError::UnsupportedVersion) => Err(AppError::bad_request(
            "unsupported_account_closure_event",
            "The Account closure event version is not supported.",
        )),
        Err(AccountClosureEventError::FutureRequest) => Err(AppError::bad_request(
            "invalid_account_closure_event",
            "The Account closure request time cannot be in the future.",
        )),
    }
}

async fn apply(db: &sqlx::PgPool, command: &AccountClosureRequestedV1) -> Result<bool, AppError> {
    let mut tx = db.begin().await?;
    let fingerprint = command.fingerprint().map_err(|error| {
        AppError::internal(
            "account_closure_event_invalid",
            format!("Account closure event could not be serialized: {error}"),
        )
    })?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO billing_account_closure_inbox
          (event_id, event_type, saga_id, principal_id, event_fingerprint)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(command.event_id)
    .bind(&command.event_type)
    .bind(command.saga_id)
    .bind(command.principal_id)
    .bind(&fingerprint)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if !inserted {
        ensure_matching_replay(&mut tx, command, &fingerprint).await?;
        tx.commit().await?;
        return Ok(false);
    }

    let owns_workspace: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM workspaces WHERE owner_user_id = $1 AND deleted_at IS NULL)",
    )
    .bind(command.principal_id)
    .fetch_one(&mut *tx)
    .await?;
    if owns_workspace {
        return Err(AppError::conflict(
            "account_owns_billing_workspaces",
            "Transfer Billing workspace ownership before closing this account.",
        ));
    }

    sqlx::query("DELETE FROM workspace_memberships WHERE principal_id = $1")
        .bind(command.principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM users WHERE principal_id = $1")
        .bind(command.principal_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE principals SET display_name = 'Deleted account', status = 'deleted', updated_at = NOW() WHERE id = $1",
    )
    .bind(command.principal_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(true)
}

async fn ensure_matching_replay(
    tx: &mut Transaction<'_, Postgres>,
    command: &AccountClosureRequestedV1,
    fingerprint: &[u8],
) -> Result<(), AppError> {
    let matches: bool = sqlx::query_scalar(
        r#"
        SELECT event_type = $2 AND saga_id = $3 AND principal_id = $4 AND event_fingerprint = $5
        FROM billing_account_closure_inbox WHERE event_id = $1
        "#,
    )
    .bind(command.event_id)
    .bind(&command.event_type)
    .bind(command.saga_id)
    .bind(command.principal_id)
    .bind(fingerprint)
    .fetch_one(&mut **tx)
    .await?;
    if !matches {
        return Err(AppError::conflict(
            "account_closure_event_collision",
            "The Account closure event identifier is already bound to another payload.",
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "billing.http.account_closure.tests.rs"]
mod tests;
