use serde::Serialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::{error::AppError, profile_models::AccountProfileRow};

pub const EVENT_TYPE: &str = "account.oidc-profile.updated.v1";

#[derive(Serialize)]
struct OidcProfileProjection<'a> {
    event_id: Uuid,
    event_type: &'static str,
    principal_id: Uuid,
    display_name: String,
    given_name: Option<&'a str>,
    family_name: Option<&'a str>,
    preferred_username: Option<&'a str>,
    birthdate: Option<chrono::NaiveDate>,
    projected_at: chrono::DateTime<chrono::Utc>,
    profile_version: i64,
}

pub async fn enqueue_tx(
    tx: &mut Transaction<'_, Postgres>,
    profile: &AccountProfileRow,
) -> Result<(), AppError> {
    let event_id = Uuid::new_v4();
    let payload = serde_json::to_value(OidcProfileProjection {
        event_id,
        event_type: EVENT_TYPE,
        principal_id: profile.principal_id,
        display_name: crate::profile_models::derive_display_name(
            profile.firstname.as_deref(),
            profile.lastname.as_deref(),
            profile.username.as_deref(),
        ),
        given_name: profile.firstname.as_deref(),
        family_name: profile.lastname.as_deref(),
        preferred_username: profile.username.as_deref(),
        birthdate: profile.birthdate,
        projected_at: profile.updated_at,
        profile_version: profile.profile_version,
    })
    .map_err(|error| {
        AppError::internal(
            "oidc_profile_event_invalid",
            format!("OIDC profile event could not be serialized: {error}"),
        )
    })?;

    sqlx::query(
        r#"
        INSERT INTO account_outbox_events (
          id, aggregate_type, aggregate_id, event_type, payload, occurred_at
        )
        VALUES ($1, 'account_profile', $2, $3, $4, $5)
        "#,
    )
    .bind(event_id)
    .bind(profile.principal_id)
    .bind(EVENT_TYPE)
    .bind(payload)
    .bind(profile.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}
