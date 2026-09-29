use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::get,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    app::AccountState,
    audit::{self, AuditInput},
    auth::Principal,
    error::{AccountError, AccountResult},
    profile::ensure_profile,
};

#[derive(Debug, Serialize, FromRow)]
struct Consent {
    id: Uuid,
    consent_type: String,
    document_version: String,
    granted_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
struct ConsentsEnvelope {
    consents: Vec<Consent>,
}

#[derive(Debug, Deserialize)]
struct ConsentInput {
    consent_type: String,
    document_version: String,
}

pub fn router(state: AccountState) -> Router {
    Router::new()
        .route("/api/v1/consents", get(list).post(grant).delete(revoke))
        .with_state(state)
}

async fn list(
    State(state): State<AccountState>,
    principal: Principal,
) -> AccountResult<Json<ConsentsEnvelope>> {
    principal.require("account:read")?;
    let consents = sqlx::query_as::<_, Consent>(
        "SELECT id, consent_type, document_version, granted_at, revoked_at FROM account_consents WHERE principal_id=$1 ORDER BY granted_at DESC, id DESC LIMIT 100",
    )
    .bind(principal.id)
    .fetch_all(&state.db)
    .await?;
    Ok(Json(ConsentsEnvelope { consents }))
}

async fn grant(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
    Json(input): Json<ConsentInput>,
) -> AccountResult<(StatusCode, Json<Consent>)> {
    principal.require("account:write")?;
    let input = validate(input)?;
    ensure_profile(&state.db, principal.id).await?;
    let id = Uuid::new_v4();
    let correlation_id = audit::correlation_id(&headers);
    let mut tx = state.db.begin().await?;
    let granted_at: DateTime<Utc> = sqlx::query_scalar(
        "INSERT INTO account_consents(id, principal_id, consent_type, document_version) VALUES($1,$2,$3,$4) RETURNING granted_at",
    )
    .bind(id)
    .bind(principal.id)
    .bind(&input.consent_type)
    .bind(&input.document_version)
    .fetch_one(&mut *tx)
    .await
    .map_err(|error| match error {
        sqlx::Error::Database(db) if db.constraint() == Some("account_consents_active_unique") => {
            AccountError::Conflict
        }
        other => AccountError::Database(other),
    })?;
    record_consent_change(
        &mut tx,
        principal.id,
        id,
        "account.consent.granted",
        "account.consent.granted.v1",
        correlation_id,
        &input,
    )
    .await?;
    tx.commit().await?;
    Ok((
        StatusCode::CREATED,
        Json(Consent {
            id,
            consent_type: input.consent_type,
            document_version: input.document_version,
            granted_at,
            revoked_at: None,
        }),
    ))
}

async fn revoke(
    State(state): State<AccountState>,
    principal: Principal,
    headers: HeaderMap,
    Json(input): Json<ConsentInput>,
) -> AccountResult<StatusCode> {
    principal.require("account:write")?;
    let input = validate(input)?;
    let correlation_id = audit::correlation_id(&headers);
    let mut tx = state.db.begin().await?;
    let id: Option<Uuid> = sqlx::query_scalar(
        "UPDATE account_consents SET revoked_at=clock_timestamp() WHERE principal_id=$1 AND consent_type=$2 AND document_version=$3 AND revoked_at IS NULL RETURNING id",
    )
    .bind(principal.id)
    .bind(&input.consent_type)
    .bind(&input.document_version)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(id) = id else {
        return Err(AccountError::NotFound);
    };
    record_consent_change(
        &mut tx,
        principal.id,
        id,
        "account.consent.revoked",
        "account.consent.revoked.v1",
        correlation_id,
        &input,
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn record_consent_change(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    principal_id: Uuid,
    consent_id: Uuid,
    event_type: &'static str,
    outbox_type: &'static str,
    correlation_id: Uuid,
    input: &ConsentInput,
) -> AccountResult<()> {
    audit::record(
        tx,
        AuditInput {
            principal_id,
            actor_principal_id: principal_id,
            event_type,
            resource_type: "consent",
            resource_id: Some(consent_id),
            correlation_id,
            details: json!({
                "consent_type": input.consent_type,
                "document_version": input.document_version
            }),
        },
    )
    .await?;
    audit::enqueue(
        tx,
        outbox_type,
        consent_id,
        json!({
            "consent_id": consent_id,
            "principal_id": principal_id,
            "consent_type": input.consent_type,
            "document_version": input.document_version
        }),
    )
    .await?;
    Ok(())
}

fn validate(input: ConsentInput) -> AccountResult<ConsentInput> {
    let consent_type = normalize("consent type", input.consent_type)?;
    let document_version = normalize("document version", input.document_version)?;
    Ok(ConsentInput {
        consent_type,
        document_version,
    })
}

fn normalize(field: &'static str, value: String) -> AccountResult<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 100 {
        return Err(AccountError::Invalid(field));
    }
    Ok(value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::{ConsentInput, validate};

    #[test]
    fn consent_values_are_trimmed_and_bounded() {
        let input = validate(ConsentInput {
            consent_type: " terms ".into(),
            document_version: " v1 ".into(),
        })
        .expect("valid");
        assert_eq!(input.consent_type, "terms");
        assert_eq!(input.document_version, "v1");
        assert!(
            validate(ConsentInput {
                consent_type: String::new(),
                document_version: "v1".into(),
            })
            .is_err()
        );
    }
}
