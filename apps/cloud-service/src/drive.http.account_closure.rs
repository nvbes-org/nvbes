use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use chrono::Utc;
use nvbes_product_account::closure_event::{AccountClosureEventError, AccountClosureRequestedV1};
use serde::Serialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{app::AppState, http::error::AppError};

#[derive(Serialize)]
struct AccountClosureResult {
    applied: bool,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/internal/v1/account-closures", post(close_account))
}

async fn close_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<AccountClosureRequestedV1>,
) -> Result<Json<AccountClosureResult>, AppError> {
    if !nvbes_core::http::internal_service::bearer_matches(&headers, &state.internal_service_token)
    {
        return Err(AppError::unauthorized(
            "invalid_internal_token",
            "A valid Cloud internal token is required.",
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
        INSERT INTO cloud_account_closure_inbox
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

    let local_user_id: Option<Uuid> = sqlx::query_scalar(
        r#"
        SELECT id FROM users
        WHERE identity_subject = $1 OR id = $2
        FOR UPDATE
        "#,
    )
    .bind(command.principal_id.to_string())
    .bind(command.principal_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(local_user_id) = local_user_id else {
        tx.commit().await?;
        return Ok(true);
    };

    let owns_workspace: bool = sqlx::query_scalar(
        r#"
        SELECT EXISTS (
          SELECT 1 FROM workspaces
          WHERE deleted_at IS NULL
            AND (owner_principal_id = $1 OR owner_user_id = $2)
        )
        "#,
    )
    .bind(command.principal_id)
    .bind(local_user_id)
    .fetch_one(&mut *tx)
    .await?;
    if owns_workspace {
        return Err(AppError::conflict(
            "account_owns_workspaces",
            "Delete or transfer owned workspaces before closing this account.",
        ));
    }

    purge_subject_access(&mut tx, local_user_id, command.principal_id).await?;
    tx.commit().await?;
    Ok(true)
}

async fn purge_subject_access(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    principal_id: Uuid,
) -> Result<(), AppError> {
    for statement in [
        "DELETE FROM sessions WHERE user_id = $1",
        "DELETE FROM mfa_factors WHERE user_id = $1",
        "DELETE FROM email_verification_tokens WHERE user_id = $1",
        "DELETE FROM password_reset_tokens WHERE user_id = $1",
        "DELETE FROM workspace_memberships WHERE user_id = $1",
        "DELETE FROM workspace_members WHERE user_id = $1",
        "DELETE FROM tenant_memberships WHERE user_id = $1",
        "DELETE FROM organization_memberships WHERE user_id = $1",
        "DELETE FROM workspace_invitations WHERE invited_by = $1",
    ] {
        sqlx::query(statement)
            .bind(user_id)
            .execute(&mut **tx)
            .await?;
    }
    sqlx::query(
        "UPDATE api_keys SET status = 'revoked' WHERE created_by_principal_id = $1 AND status = 'active'",
    )
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "UPDATE share_links SET revoked_at = COALESCE(revoked_at, NOW()), updated_at = NOW() WHERE created_by_principal_id = $1",
    )
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        r#"
        UPDATE users
        SET email = $2, display_name = 'Deleted account', password_hash = '',
            email_verified_at = NULL, mfa_enabled = FALSE,
            status = 'deleted', deleted_at = COALESCE(deleted_at, NOW()), updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(format!("closed-{user_id}@deleted.invalid"))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn ensure_matching_replay(
    tx: &mut Transaction<'_, Postgres>,
    command: &AccountClosureRequestedV1,
    fingerprint: &[u8],
) -> Result<(), AppError> {
    let matches: bool = sqlx::query_scalar(
        r#"
        SELECT event_type = $2 AND saga_id = $3 AND principal_id = $4 AND event_fingerprint = $5
        FROM cloud_account_closure_inbox WHERE event_id = $1
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
#[path = "drive.http.account_closure.tests.rs"]
mod tests;
