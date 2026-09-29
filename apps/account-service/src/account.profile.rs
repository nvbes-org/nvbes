use axum::{Json, Router, extract::State, http::HeaderMap, routing::get};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use crate::{
    app::AccountState,
    audit::{self, AuditInput},
    auth::Principal,
    error::{AccountError, AccountResult},
};

#[derive(Debug, Clone, FromRow)]
pub(crate) struct ProfileRow {
    principal_id: Uuid,
    firstname: Option<String>,
    lastname: Option<String>,
    username: Option<String>,
    birthdate: Option<NaiveDate>,
    region: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
struct ProfileEnvelope {
    user: Profile,
}

#[derive(Debug, Serialize)]
struct Profile {
    id: Uuid,
    display_name: String,
    firstname: Option<String>,
    lastname: Option<String>,
    username: Option<String>,
    birthdate: Option<NaiveDate>,
    region: Option<String>,
    created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct UpdateProfile {
    firstname: Option<String>,
    lastname: Option<String>,
    username: Option<String>,
    birthdate: Option<NaiveDate>,
    region: Option<String>,
}

pub fn router(state: AccountState) -> Router {
    Router::new()
        .route("/api/v1/profile", get(get_profile).put(update_profile))
        .with_state(state)
}

async fn get_profile(
    State(state): State<AccountState>,
    principal: Principal,
) -> AccountResult<Json<ProfileEnvelope>> {
    principal.require("account:read")?;
    Ok(Json(ProfileEnvelope {
        user: ensure_profile(&state.db, principal.id).await?.into(),
    }))
}

async fn update_profile(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
    Json(input): Json<UpdateProfile>,
) -> AccountResult<Json<ProfileEnvelope>> {
    principal.require("account:write")?;
    validate_profile(&input)?;
    ensure_profile(&state.db, principal.id).await?;
    let username = input
        .username
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_lowercase);
    let mut tx = state.db.begin().await?;
    let row = sqlx::query_as::<_, ProfileRow>(
        "UPDATE account_profiles SET firstname=$2, lastname=$3, username=$4, birthdate=$5, region=$6, updated_at=clock_timestamp() WHERE principal_id=$1 AND lifecycle_status='active' RETURNING principal_id, firstname, lastname, username, birthdate, region, created_at",
    )
    .bind(principal.id)
    .bind(clean(input.firstname))
    .bind(clean(input.lastname))
    .bind(username)
    .bind(input.birthdate)
    .bind(clean(input.region))
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AccountError::Conflict)?;
    audit::record(&mut tx, AuditInput {
        principal_id: principal.id,
        actor_principal_id: principal.id,
        event_type: "account.profile.updated",
        resource_type: "profile",
        resource_id: Some(principal.id),
        correlation_id: audit::correlation_id(&headers),
        details: json!({"fields": ["firstname", "lastname", "username", "birthdate", "region"]}),
    }).await?;
    tx.commit().await?;
    Ok(Json(ProfileEnvelope { user: row.into() }))
}

pub async fn ensure_profile(db: &PgPool, principal_id: Uuid) -> AccountResult<ProfileRow> {
    let mut tx = db.begin().await?;
    let row = ensure_profile_in_transaction(&mut tx, principal_id).await?;
    tx.commit().await?;
    Ok(row)
}

pub(crate) async fn ensure_profile_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal_id: Uuid,
) -> AccountResult<ProfileRow> {
    let row = sqlx::query_as::<_, ProfileRow>("INSERT INTO account_profiles(principal_id) VALUES($1) ON CONFLICT(principal_id) DO NOTHING RETURNING principal_id, firstname, lastname, username, birthdate, region, created_at")
        .bind(principal_id).fetch_optional(&mut **tx).await?;
    if row.is_some() {
        sqlx::query(
            "INSERT INTO account_preferences(principal_id) VALUES($1) ON CONFLICT DO NOTHING",
        )
        .bind(principal_id)
        .execute(&mut **tx)
        .await?;
        audit::record(
            tx,
            AuditInput {
                principal_id,
                actor_principal_id: principal_id,
                event_type: "account.profile.created",
                resource_type: "profile",
                resource_id: Some(principal_id),
                correlation_id: Uuid::new_v4(),
                details: json!({"source": "authenticated_first_access"}),
            },
        )
        .await?;
        audit::enqueue(
            tx,
            "account.profile.created.v1",
            principal_id,
            json!({"principal_id": principal_id}),
        )
        .await?;
    }
    match row {
        Some(value) => Ok(value),
        None => sqlx::query_as("SELECT principal_id, firstname, lastname, username, birthdate, region, created_at FROM account_profiles WHERE principal_id=$1").bind(principal_id).fetch_one(&mut **tx).await.map_err(Into::into),
    }
}

fn validate_profile(input: &UpdateProfile) -> AccountResult<()> {
    for value in [&input.firstname, &input.lastname] {
        if value.as_ref().is_some_and(|v| v.trim().len() > 100) {
            return Err(AccountError::Invalid("name exceeds 100 characters"));
        }
    }
    if input
        .username
        .as_ref()
        .is_some_and(|v| !(3..=100).contains(&v.trim().len()))
    {
        return Err(AccountError::Invalid("username length"));
    }
    if input
        .region
        .as_ref()
        .is_some_and(|v| !(2..=32).contains(&v.trim().len()))
    {
        return Err(AccountError::Invalid("region length"));
    }
    if input
        .birthdate
        .is_some_and(|date| date > Utc::now().date_naive())
    {
        return Err(AccountError::Invalid("birthdate is in the future"));
    }
    Ok(())
}

fn clean(value: Option<String>) -> Option<String> {
    value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

impl From<ProfileRow> for Profile {
    fn from(row: ProfileRow) -> Self {
        let display_name = [row.firstname.as_deref(), row.lastname.as_deref()]
            .into_iter()
            .flatten()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>()
            .join(" ");
        let display_name = if display_name.is_empty() {
            row.username.clone().unwrap_or_else(|| "User".into())
        } else {
            display_name
        };
        Self {
            id: row.principal_id,
            display_name,
            firstname: row.firstname,
            lastname: row.lastname,
            username: row.username,
            birthdate: row.birthdate,
            region: row.region,
            created_at: row.created_at,
        }
    }
}
