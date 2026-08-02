use axum::{Json, Router, extract::State, http::HeaderMap, routing::post};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{app::AppState, http::error::AppError};

pub const EVENT_TYPE: &str = "account.oidc-profile.updated.v1";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct OidcProfileProjection {
    event_id: Uuid,
    event_type: String,
    principal_id: Uuid,
    display_name: String,
    given_name: Option<String>,
    family_name: Option<String>,
    preferred_username: Option<String>,
    birthdate: Option<NaiveDate>,
    projected_at: DateTime<Utc>,
    profile_version: i64,
}

#[derive(serde::Serialize)]
struct ProjectionResult {
    applied: bool,
}

pub struct OidcProfileClaims {
    pub display_name: String,
    pub given_name: Option<String>,
    pub family_name: Option<String>,
    pub preferred_username: Option<String>,
    pub birthdate: Option<NaiveDate>,
    pub projected_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/oidc-profile-projections", post(project_profile))
}

async fn project_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut projection): Json<OidcProfileProjection>,
) -> Result<Json<ProjectionResult>, AppError> {
    if !nvbes_core::http::internal_service::bearer_matches(&headers, &state.identity_internal_token)
    {
        return Err(AppError::unauthorized(
            "invalid_internal_token",
            "A valid Identity internal token is required.",
        ));
    }
    projection.validate()?;
    let applied = apply(&state.db, projection).await?;
    Ok(Json(ProjectionResult { applied }))
}

impl OidcProfileProjection {
    fn validate(&mut self) -> Result<(), AppError> {
        if self.event_type != EVENT_TYPE {
            return Err(AppError::bad_request(
                "unsupported_oidc_profile_event",
                "The OIDC profile event version is not supported.",
            ));
        }
        self.display_name = required_text("display_name", &self.display_name, 201)?;
        self.given_name = optional_text("given_name", self.given_name.take(), 100)?;
        self.family_name = optional_text("family_name", self.family_name.take(), 100)?;
        self.preferred_username =
            optional_text("preferred_username", self.preferred_username.take(), 100)?;
        if self
            .birthdate
            .is_some_and(|date| date > Utc::now().date_naive())
        {
            return Err(AppError::bad_request(
                "invalid_oidc_profile",
                "The OIDC birthdate cannot be in the future.",
            ));
        }
        if self.profile_version < 0 {
            return Err(AppError::bad_request(
                "invalid_oidc_profile",
                "The OIDC profile version cannot be negative.",
            ));
        }
        Ok(())
    }
}

async fn apply(db: &PgPool, projection: OidcProfileProjection) -> Result<bool, AppError> {
    let mut tx = db.begin().await?;
    let fingerprint = event_fingerprint(&projection)?;
    let inserted = sqlx::query(
        r#"
        INSERT INTO identity_inbox_events (
          event_id, event_type, principal_id, event_fingerprint
        )
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (event_id) DO NOTHING
        "#,
    )
    .bind(projection.event_id)
    .bind(&projection.event_type)
    .bind(projection.principal_id)
    .bind(&fingerprint)
    .execute(&mut *tx)
    .await?
    .rows_affected()
        == 1;
    if !inserted {
        ensure_matching_replay(&mut tx, &projection, &fingerprint).await?;
        tx.commit().await?;
        return Ok(false);
    }

    let result = sqlx::query(
        r#"
        INSERT INTO identity_oidc_profile_claims (
          principal_id, display_name, given_name, family_name, preferred_username,
          birthdate, projected_at, profile_version
        )
        SELECT $1, $2, $3, $4, $5, $6, $7, $8
        FROM principals
        WHERE id = $1 AND status = 'active'
        ON CONFLICT (principal_id) DO UPDATE
        SET display_name = EXCLUDED.display_name,
            given_name = EXCLUDED.given_name,
            family_name = EXCLUDED.family_name,
            preferred_username = EXCLUDED.preferred_username,
            birthdate = EXCLUDED.birthdate,
            projected_at = EXCLUDED.projected_at,
            profile_version = EXCLUDED.profile_version
        WHERE EXCLUDED.profile_version > identity_oidc_profile_claims.profile_version
        "#,
    )
    .bind(projection.principal_id)
    .bind(projection.display_name)
    .bind(projection.given_name)
    .bind(projection.family_name)
    .bind(projection.preferred_username)
    .bind(projection.birthdate)
    .bind(projection.projected_at)
    .bind(projection.profile_version)
    .execute(&mut *tx)
    .await?;
    if result.rows_affected() == 0 {
        let principal_active: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM principals WHERE id = $1 AND status = 'active')",
        )
        .bind(projection.principal_id)
        .fetch_one(&mut *tx)
        .await?;
        if !principal_active {
            return Err(AppError::conflict(
                "identity_principal_inactive",
                "The Identity principal does not exist or is not active.",
            ));
        }
        tx.commit().await?;
        return Ok(false);
    }
    tx.commit().await?;
    Ok(true)
}

fn event_fingerprint(projection: &OidcProfileProjection) -> Result<Vec<u8>, AppError> {
    use sha2::{Digest, Sha256};

    let payload = serde_json::to_vec(projection).map_err(|error| {
        AppError::internal(
            "oidc_profile_event_invalid",
            format!("OIDC profile event could not be serialized: {error}"),
        )
    })?;
    Ok(Sha256::digest(payload).to_vec())
}

async fn ensure_matching_replay(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    projection: &OidcProfileProjection,
    fingerprint: &[u8],
) -> Result<(), AppError> {
    let matches: bool = sqlx::query_scalar(
        r#"
        SELECT event_type = $2
          AND principal_id = $3
          AND event_fingerprint = $4
        FROM identity_inbox_events
        WHERE event_id = $1
        "#,
    )
    .bind(projection.event_id)
    .bind(&projection.event_type)
    .bind(projection.principal_id)
    .bind(fingerprint)
    .fetch_one(&mut **tx)
    .await?;
    if !matches {
        return Err(AppError::conflict(
            "oidc_profile_event_collision",
            "The OIDC profile event identifier is already bound to another payload.",
        ));
    }
    Ok(())
}

pub async fn fetch_claims(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<Option<OidcProfileClaims>, AppError> {
    use sqlx::Row;

    let row = sqlx::query(
        r#"
        SELECT display_name, given_name, family_name, preferred_username, birthdate, projected_at
        FROM identity_oidc_profile_claims
        WHERE principal_id = $1
        "#,
    )
    .bind(principal_id)
    .fetch_optional(db)
    .await?;
    Ok(row.map(|row| OidcProfileClaims {
        display_name: row.get("display_name"),
        given_name: row.get("given_name"),
        family_name: row.get("family_name"),
        preferred_username: row.get("preferred_username"),
        birthdate: row.get("birthdate"),
        projected_at: row.get("projected_at"),
    }))
}

fn required_text(field: &'static str, value: &str, maximum: usize) -> Result<String, AppError> {
    optional_text(field, Some(value.to_string()), maximum)?.ok_or_else(|| {
        AppError::bad_request("invalid_oidc_profile", format!("`{field}` is required."))
    })
}

fn optional_text(
    field: &'static str,
    value: Option<String>,
    maximum: usize,
) -> Result<Option<String>, AppError> {
    value
        .map(|value| {
            let value = value.trim();
            if value.is_empty() || value.chars().count() > maximum {
                return Err(AppError::bad_request(
                    "invalid_oidc_profile",
                    format!("`{field}` must contain between 1 and {maximum} characters."),
                ));
            }
            Ok(value.to_string())
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use super::{EVENT_TYPE, OidcProfileProjection};

    #[test]
    fn profile_projection_normalizes_and_versions_its_contract() {
        let mut projection = OidcProfileProjection {
            event_id: Uuid::new_v4(),
            event_type: EVENT_TYPE.to_string(),
            principal_id: Uuid::new_v4(),
            display_name: " Ada Lovelace ".to_string(),
            given_name: Some(" Ada ".to_string()),
            family_name: Some(" Lovelace ".to_string()),
            preferred_username: Some(" ada ".to_string()),
            birthdate: None,
            projected_at: Utc::now(),
            profile_version: 1,
        };
        projection.validate().expect("valid OIDC projection");
        assert_eq!(projection.display_name, "Ada Lovelace");
        assert_eq!(projection.preferred_username.as_deref(), Some("ada"));

        projection.event_type = "account.oidc-profile.updated.v2".to_string();
        assert!(projection.validate().is_err());
    }
}
