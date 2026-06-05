use super::db::{self, UserConsent};
use crate::http::error::AppError;
use nvbes_audit::{AuditEventInput, insert_audit_event_pool};
use sqlx::PgPool;
use uuid::Uuid;

/// Records a new legal consent for a user, masks their IP, and writes a compliance audit log event.
pub async fn grant_consent(
    db: &PgPool,
    principal_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    consent_type: &str,
    document_version: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<UserConsent, AppError> {
    // 1. Persist the consent to the database
    let consent =
        db::record_consent(db, principal_id, consent_type, document_version, ip_address).await?;

    // 2. Anonymize the IP address for the audit log to follow Option 1 (Partial Masking)
    let masked_ip = ip_address.and_then(db::anonymize_ip_str);

    // 3. Write a compliant append-only audit trail event
    let metadata = serde_json::json!({
        "consent_id": consent.id,
        "consent_type": consent_type,
        "document_version": document_version,
        "action_taken": "granted"
    });

    let audit_input = AuditEventInput {
        tenant_id,
        workspace_id,
        actor_principal_id: Some(principal_id),
        action: "USER_CONSENT_GRANTED",
        target_type: "legal_consent",
        target_id: Some(consent.id),
        ip: masked_ip.as_deref(),
        user_agent,
        metadata,
    };

    if let Err(e) = insert_audit_event_pool(db, audit_input).await {
        // Log audit event failure but don't break the user experience
        tracing::error!("Failed to log USER_CONSENT_GRANTED audit event: {:?}", e);
    }

    Ok(consent)
}

/// Revokes an existing legal consent for a user and writes a compliance audit log event.
pub async fn revoke_consent(
    db: &PgPool,
    principal_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    consent_type: &str,
    document_version: &str,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<(), AppError> {
    // 1. Revoke the consent in the database
    db::revoke_consent(db, principal_id, consent_type, document_version).await?;

    // 2. Anonymize IP address for the audit log
    let masked_ip = ip_address.and_then(db::anonymize_ip_str);

    // 3. Write a compliance audit trail event
    let metadata = serde_json::json!({
        "consent_type": consent_type,
        "document_version": document_version,
        "action_taken": "revoked"
    });

    let audit_input = AuditEventInput {
        tenant_id,
        workspace_id,
        actor_principal_id: Some(principal_id),
        action: "USER_CONSENT_REVOKED",
        target_type: "legal_consent",
        target_id: None,
        ip: masked_ip.as_deref(),
        user_agent,
        metadata,
    };

    if let Err(e) = insert_audit_event_pool(db, audit_input).await {
        tracing::error!("Failed to log USER_CONSENT_REVOKED audit event: {:?}", e);
    }

    Ok(())
}

/// Lists all active consents for a principal.
pub async fn get_active_consents(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<Vec<UserConsent>, AppError> {
    db::list_active_consents(db, principal_id).await
}

/// Auto-records a GPC opt-out consent when the Sec-GPC header is present.
/// Returns Ok(true) if a new consent was recorded, Ok(false) if already active.
pub async fn record_gpc_opt_out(
    db: &PgPool,
    principal_id: Uuid,
    tenant_id: Uuid,
    workspace_id: Option<Uuid>,
    ip_address: Option<&str>,
    user_agent: Option<&str>,
) -> Result<bool, AppError> {
    let existing = db::is_consent_active(db, principal_id, "gpc_opt_out", "gpc_v1").await?;

    if existing {
        return Ok(false);
    }

    grant_consent(
        db,
        principal_id,
        tenant_id,
        workspace_id,
        "gpc_opt_out",
        "gpc_v1",
        ip_address,
        user_agent,
    )
    .await?;

    Ok(true)
}
