use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use chrono::Utc;
use nvbes_product_account::closure_event::{
    ACCOUNT_CLOSURE_REQUESTED_V1, AccountClosureEventError,
    AccountClosureRequestedV1 as AccountClosureCommand,
};
use serde::Serialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{app::AppState, http::error::AppError};

pub const EVENT_TYPE: &str = ACCOUNT_CLOSURE_REQUESTED_V1;

#[derive(Serialize)]
struct AccountClosureResult {
    applied: bool,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/account-closures", post(close_account))
}

async fn close_account(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(command): Json<AccountClosureCommand>,
) -> Result<Json<AccountClosureResult>, AppError> {
    require_internal_token(&headers, &state.identity_internal_token)?;
    validate_command(&command)?;

    crate::domains::oauth::security_events::enqueue_all_sessions_revoked(
        &state.db,
        &state.redis,
        command.principal_id,
        None,
    )
    .await?;

    let applied = apply(&state.db, &command).await?;
    clear_cached_credentials(&state.redis, command.principal_id).await?;
    Ok(Json(AccountClosureResult { applied }))
}

fn require_internal_token(headers: &HeaderMap, expected: &str) -> Result<(), AppError> {
    if !nvbes_core::http::internal_service::bearer_matches(headers, expected) {
        return Err(AppError::unauthorized(
            "invalid_internal_token",
            "A valid Identity internal token is required.",
        ));
    }
    Ok(())
}

fn validate_command(command: &AccountClosureCommand) -> Result<(), AppError> {
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

async fn apply(db: &PgPool, command: &AccountClosureCommand) -> Result<bool, AppError> {
    let mut tx = db.begin().await?;
    let fingerprint = event_fingerprint(command)?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO identity_inbox_events (
          event_id, event_type, principal_id, event_fingerprint
        )
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(command.event_id)
    .bind(&command.event_type)
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

    let principal_kind: Option<String> =
        sqlx::query_scalar("SELECT principal_kind::text FROM principals WHERE id = $1 FOR UPDATE")
            .bind(command.principal_id)
            .fetch_optional(&mut *tx)
            .await?;
    if principal_kind
        .as_deref()
        .is_some_and(|kind| kind != "human")
    {
        return Err(AppError::conflict(
            "account_closure_subject_invalid",
            "Only human Identity principals can be closed through Account.",
        ));
    }

    if principal_kind.is_some() {
        purge_authentication_state(&mut tx, command.principal_id).await?;
    }
    tx.commit().await?;
    Ok(true)
}

async fn purge_authentication_state(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
) -> Result<(), AppError> {
    crate::domains::auth::sessions_mgmt::revoke_all_user_sessions_tx(tx, principal_id).await?;
    for statement in [
        "DELETE FROM registration_enrollment_tokens WHERE principal_id = $1",
        "DELETE FROM enterprise_password_recovery_requests WHERE principal_id = $1",
        "DELETE FROM privileged_action_approvals WHERE requested_by = $1 OR approved_by = $1",
        "DELETE FROM privileged_access_grants WHERE principal_id = $1",
        "DELETE FROM tenant_break_glass_accounts WHERE principal_id = $1",
        "DELETE FROM developer_token_debug_sessions WHERE actor_principal_id = $1",
        "DELETE FROM developer_role_assignments WHERE principal_id = $1",
        "DELETE FROM access_review_reminders WHERE recipient_principal_id = $1",
        "DELETE FROM workspace_memberships WHERE principal_id = $1",
        "DELETE FROM organization_memberships WHERE principal_id = $1",
        "DELETE FROM tenant_memberships WHERE principal_id = $1",
        "DELETE FROM password_history WHERE principal_id = $1",
        "DELETE FROM mfa_factors WHERE principal_id = $1",
        "DELETE FROM user_identities WHERE principal_id = $1",
        "DELETE FROM user_email_addresses WHERE principal_id = $1",
        "DELETE FROM oauth_consents WHERE principal_id = $1",
        "DELETE FROM account_devices WHERE principal_id = $1",
        "DELETE FROM identity_oidc_profile_claims WHERE principal_id = $1",
        "DELETE FROM user_consents WHERE principal_id = $1",
    ] {
        sqlx::query(statement)
            .bind(principal_id)
            .execute(&mut **tx)
            .await?;
    }

    sqlx::query(
        r#"
        UPDATE users
        SET email = $2,
            password_hash = NULL,
            email_verified_at = NULL,
            password_last_changed_at = NULL,
            preferences = '{}'::jsonb,
            notifications = '{}'::jsonb,
            profile_avatar_key = NULL,
            status = 'deleted',
            updated_at = NOW()
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .bind(format!("closed-{principal_id}@deleted.invalid"))
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        r#"
        UPDATE principals
        SET status = 'deleted', display_name = 'Deleted account', updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(principal_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn clear_cached_credentials(
    redis: &nvbes_redis::RedisPool,
    principal_id: Uuid,
) -> Result<(), AppError> {
    nvbes_redis::session::clear_user_sessions(redis, &principal_id.to_string())
        .await
        .map_err(|error| AppError::internal("redis_session_revoke_failed", error.to_string()))?;
    nvbes_redis::refresh_token::revoke_all_user_refresh_tokens(redis, principal_id)
        .await
        .map_err(|error| AppError::internal("refresh_token_revoke_failed", error.to_string()))
}

fn event_fingerprint(command: &AccountClosureCommand) -> Result<Vec<u8>, AppError> {
    command.fingerprint().map_err(|error| {
        AppError::internal(
            "account_closure_event_invalid",
            format!("Account closure event could not be serialized: {error}"),
        )
    })
}

async fn ensure_matching_replay(
    tx: &mut Transaction<'_, Postgres>,
    command: &AccountClosureCommand,
    fingerprint: &[u8],
) -> Result<(), AppError> {
    let matches: bool = sqlx::query_scalar(
        r#"
        SELECT event_type = $2 AND principal_id = $3 AND event_fingerprint = $4
        FROM identity_inbox_events
        WHERE event_id = $1
        "#,
    )
    .bind(command.event_id)
    .bind(&command.event_type)
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
#[path = "identity.domains.auth.account_closure.tests.rs"]
mod tests;
