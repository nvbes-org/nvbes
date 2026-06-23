use axum::{
    Json, Router,
    extract::{Path, State},
    http::HeaderMap,
    routing::post,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::app::AppState;
use crate::backoffice_authorization::{
    BackofficePermission, require_confirmation, require_permission,
};
use crate::billing_admin_access::actor_principal_id;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
struct SecurityActionRequest {
    confirm_code: String,
    reason: String,
}

#[derive(Debug, Serialize)]
struct SecurityActionResult {
    object_id: Uuid,
    tenant_id: Uuid,
    principal_id: Uuid,
    audit_action: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/admin/security-center/mfa-factors/{factorId}/revoke",
            post(revoke_mfa_factor_route),
        )
        .route(
            "/admin/security-center/oauth-consents/{consentId}/revoke",
            post(revoke_oauth_consent_route),
        )
}

async fn revoke_mfa_factor_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(factor_id): Path<Uuid>,
    Json(request): Json<SecurityActionRequest>,
) -> Result<Json<SecurityActionResult>, AppError> {
    require_permission(&headers, BackofficePermission::SecurityMutate)?;
    require_confirmation(&request.confirm_code, "REVOKE MFA")?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        revoke_mfa_factor(&state.db, actor_id, factor_id, request).await?,
    ))
}

async fn revoke_oauth_consent_route(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(consent_id): Path<Uuid>,
    Json(request): Json<SecurityActionRequest>,
) -> Result<Json<SecurityActionResult>, AppError> {
    require_permission(&headers, BackofficePermission::SecurityMutate)?;
    require_confirmation(&request.confirm_code, "REVOKE OAUTH CONSENT")?;
    let actor_id = actor_principal_id(&headers)?;
    Ok(Json(
        revoke_oauth_consent(&state.db, actor_id, consent_id, request).await?,
    ))
}

async fn revoke_mfa_factor(
    db: &PgPool,
    actor_id: Uuid,
    factor_id: Uuid,
    request: SecurityActionRequest,
) -> Result<SecurityActionResult, AppError> {
    validate_security_action_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
        SELECT mf.principal_id, mf.status::text AS status, p.tenant_id
        FROM mfa_factors mf
        JOIN principals p ON p.id = mf.principal_id
        WHERE mf.id = $1
        FOR UPDATE OF mf
        "#,
    )
    .bind(factor_id)
    .fetch_one(tx.as_mut())
    .await?;
    let principal_id: Uuid = row.get("principal_id");
    let tenant_id: Uuid = row.get("tenant_id");
    let status: String = row.get("status");
    if status == "revoked" {
        return Err(AppError::bad_request(
            "mfa_factor_already_revoked",
            "MFA factor is already revoked.",
        ));
    }

    sqlx::query("UPDATE mfa_factors SET status = 'revoked' WHERE id = $1")
        .bind(factor_id)
        .execute(tx.as_mut())
        .await?;
    insert_security_audit(
        tx.as_mut(),
        tenant_id,
        actor_id,
        "internal_admin.security.mfa_factor.revoked",
        "mfa_factor",
        factor_id,
        principal_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(SecurityActionResult {
        object_id: factor_id,
        tenant_id,
        principal_id,
        audit_action: "internal_admin.security.mfa_factor.revoked",
    })
}

async fn revoke_oauth_consent(
    db: &PgPool,
    actor_id: Uuid,
    consent_id: Uuid,
    request: SecurityActionRequest,
) -> Result<SecurityActionResult, AppError> {
    validate_security_action_reason(&request.reason)?;

    let mut tx = db.begin().await?;
    let row = sqlx::query(
        "SELECT tenant_id, principal_id, revoked_at FROM oauth_consents WHERE id = $1 FOR UPDATE",
    )
    .bind(consent_id)
    .fetch_one(tx.as_mut())
    .await?;
    let tenant_id: Uuid = row.get("tenant_id");
    let principal_id: Uuid = row.get("principal_id");
    let revoked_at: Option<DateTime<Utc>> = row.get("revoked_at");
    if revoked_at.is_some() {
        return Err(AppError::bad_request(
            "oauth_consent_already_revoked",
            "OAuth consent is already revoked.",
        ));
    }

    sqlx::query("UPDATE oauth_consents SET revoked_at = NOW() WHERE id = $1")
        .bind(consent_id)
        .execute(tx.as_mut())
        .await?;
    insert_security_audit(
        tx.as_mut(),
        tenant_id,
        actor_id,
        "internal_admin.security.oauth_consent.revoked",
        "oauth_consent",
        consent_id,
        principal_id,
        &request.reason,
    )
    .await?;
    tx.commit().await?;
    Ok(SecurityActionResult {
        object_id: consent_id,
        tenant_id,
        principal_id,
        audit_action: "internal_admin.security.oauth_consent.revoked",
    })
}

async fn insert_security_audit(
    executor: &mut sqlx::PgConnection,
    tenant_id: Uuid,
    actor_id: Uuid,
    action: &'static str,
    target_type: &str,
    target_id: Uuid,
    principal_id: Uuid,
    reason: &str,
) -> Result<(), AppError> {
    sqlx::query(
        "INSERT INTO audit_events (
           tenant_id, actor_principal_id, action, target_type, target_id, metadata, event_hash
         ) VALUES (
           $1, $2, $3, $4, $5,
           jsonb_build_object('reason', $6, 'principal_id', $7::text),
           gen_random_uuid()::text
         )",
    )
    .bind(tenant_id)
    .bind(actor_id)
    .bind(action)
    .bind(target_type)
    .bind(target_id)
    .bind(reason.trim())
    .bind(principal_id)
    .execute(executor)
    .await?;
    Ok(())
}

fn validate_security_action_reason(reason: &str) -> Result<(), AppError> {
    if reason.trim().len() < 12 {
        return Err(AppError::bad_request(
            "audit_reason_required",
            "Security actions require a detailed audit reason.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn security_action_reason_must_be_detailed() {
        assert!(validate_security_action_reason("short").is_err());
        assert!(validate_security_action_reason("incident SEC-456 approved").is_ok());
    }
}
