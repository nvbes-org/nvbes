use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;

use crate::{
    app::AccountState,
    audit::{self, AuditInput},
    auth::Principal,
    error::{AccountError, AccountResult},
    profile::ensure_profile,
};

#[derive(Debug, Serialize, FromRow)]
struct Preferences {
    theme: String,
    language: String,
}

#[derive(Debug, Deserialize)]
struct UpdatePreferences {
    theme: String,
    language: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
struct Notifications {
    #[sqlx(rename = "email_notifications")]
    email: bool,
    #[sqlx(rename = "push_notifications")]
    push: bool,
    #[sqlx(rename = "in_app_notifications")]
    in_app: bool,
    marketing_email: bool,
}

pub fn router(state: AccountState) -> Router {
    Router::new()
        .route(
            "/api/v1/preferences",
            get(get_preferences).put(update_preferences),
        )
        .route(
            "/api/v1/notifications",
            get(get_notifications).put(update_notifications),
        )
        .with_state(state)
}

async fn get_preferences(
    State(state): State<AccountState>,
    principal: Principal,
) -> AccountResult<Json<Preferences>> {
    principal.require("account:read")?;
    ensure_profile(&state.db, principal.id).await?;
    let value =
        sqlx::query_as("SELECT theme,language FROM account_preferences WHERE principal_id=$1")
            .bind(principal.id)
            .fetch_one(&state.db)
            .await?;
    Ok(Json(value))
}

async fn update_preferences(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
    Json(input): Json<UpdatePreferences>,
) -> AccountResult<Json<Preferences>> {
    principal.require("account:write")?;
    if !matches!(input.theme.as_str(), "system" | "light" | "dark") {
        return Err(AccountError::Invalid("unsupported theme"));
    }
    if !matches!(input.language.as_str(), "fr" | "en") {
        return Err(AccountError::Invalid("unsupported language"));
    }
    ensure_profile(&state.db, principal.id).await?;
    let mut tx = state.db.begin().await?;
    let value = sqlx::query_as("UPDATE account_preferences SET theme=$2,language=$3,updated_at=clock_timestamp() WHERE principal_id=$1 RETURNING theme,language")
        .bind(principal.id)
        .bind(input.theme)
        .bind(input.language)
        .fetch_one(&mut *tx)
        .await?;
    record_change(&mut tx, &principal, &headers, "account.preferences.updated").await?;
    tx.commit().await?;
    Ok(Json(value))
}

async fn get_notifications(
    State(state): State<AccountState>,
    principal: Principal,
) -> AccountResult<Json<Notifications>> {
    principal.require("account:read")?;
    ensure_profile(&state.db, principal.id).await?;
    let value = sqlx::query_as("SELECT email_notifications,push_notifications,in_app_notifications,marketing_email FROM account_preferences WHERE principal_id=$1")
        .bind(principal.id)
        .fetch_one(&state.db)
        .await?;
    Ok(Json(value))
}

async fn update_notifications(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
    Json(input): Json<Notifications>,
) -> AccountResult<Json<Notifications>> {
    principal.require("account:write")?;
    ensure_profile(&state.db, principal.id).await?;
    let mut tx = state.db.begin().await?;
    let value = sqlx::query_as("UPDATE account_preferences SET email_notifications=$2,push_notifications=$3,in_app_notifications=$4,marketing_email=$5,updated_at=clock_timestamp() WHERE principal_id=$1 RETURNING email_notifications,push_notifications,in_app_notifications,marketing_email")
        .bind(principal.id)
        .bind(input.email)
        .bind(input.push)
        .bind(input.in_app)
        .bind(input.marketing_email)
        .fetch_one(&mut *tx)
        .await?;
    record_change(
        &mut tx,
        &principal,
        &headers,
        "account.notifications.updated",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(value))
}

async fn record_change(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal: &Principal,
    headers: &HeaderMap,
    event_type: &'static str,
) -> AccountResult<()> {
    audit::record(
        tx,
        AuditInput {
            principal_id: principal.id,
            actor_principal_id: principal.id,
            event_type,
            resource_type: "preferences",
            resource_id: Some(principal.id),
            correlation_id: audit::correlation_id(headers),
            details: json!({}),
        },
    )
    .await?;
    Ok(())
}
