use super::db::{self, UserConsent};
use crate::http::error::AppError;
use nvbes_core::pagination::{KeysetCursor, decode_cursor, encode_cursor, page_from_rows};
use sqlx::PgPool;
use uuid::Uuid;

const NON_REVOCABLE_CONSENT_TYPES: [&str; 3] = [
    "terms_of_service",
    "data_processing_agreement",
    "privacy_policy",
];

fn is_revocable_consent_type(consent_type: &str) -> bool {
    !NON_REVOCABLE_CONSENT_TYPES.contains(&consent_type)
}

/// Records a new legal consent for a user, masks their IP, and writes a compliance audit log event.
#[expect(
    clippy::too_many_arguments,
    reason = "Consent recording keeps legal, tenant, workspace, and request metadata explicit."
)]
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

    let audit_input = crate::domains::audit::AuditRecordInput {
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

    if let Err(e) = crate::domains::audit::record_event(db, audit_input).await {
        // Log audit event failure but don't break the user experience
        tracing::error!("Failed to log USER_CONSENT_GRANTED audit event: {:?}", e);
    }

    Ok(consent)
}

/// Revokes an existing legal consent for a user and writes a compliance audit log event.
#[expect(
    clippy::too_many_arguments,
    reason = "Consent revocation keeps legal, tenant, workspace, and request metadata explicit."
)]
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
    if !is_revocable_consent_type(consent_type) {
        return Err(AppError::forbidden(
            "consent_not_revocable",
            "This required legal agreement cannot be revoked.",
        ));
    }

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

    let audit_input = crate::domains::audit::AuditRecordInput {
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

    if let Err(e) = crate::domains::audit::record_event(db, audit_input).await {
        tracing::error!("Failed to log USER_CONSENT_REVOKED audit event: {:?}", e);
    }

    Ok(())
}

/// Lists all consent records for a principal, including revoked entries.
pub async fn get_consents(db: &PgPool, principal_id: Uuid) -> Result<Vec<UserConsent>, AppError> {
    db::list_consents(db, principal_id, None, i64::MAX).await
}

/// Backwards-compatible alias for callers that still expect the old name.
pub async fn get_active_consents(
    db: &PgPool,
    principal_id: Uuid,
) -> Result<Vec<UserConsent>, AppError> {
    get_consents(db, principal_id).await
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct ConsentHistoryResult {
    pub consents: Vec<UserConsent>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

pub async fn list_consents(
    db: &PgPool,
    principal_id: Uuid,
    limit: Option<i64>,
    cursor: Option<String>,
) -> Result<ConsentHistoryResult, AppError> {
    let limit = limit.unwrap_or(50).clamp(1, 200);
    let cursor = cursor
        .as_deref()
        .map(decode_cursor::<KeysetCursor>)
        .transpose()
        .map_err(|_| AppError::bad_request("invalid_cursor", "Pagination cursor is invalid."))?;
    let consents = db::list_consents(db, principal_id, cursor.as_ref(), limit + 1).await?;
    let page = page_from_rows(consents, limit as usize, |consent| KeysetCursor {
        created_at: consent.granted_at,
        id: consent.id,
    });

    Ok(ConsentHistoryResult {
        consents: page.items,
        next_cursor: page
            .next_cursor
            .as_ref()
            .map(encode_cursor)
            .transpose()
            .map_err(|_| {
                AppError::internal(
                    "cursor_encoding_failed",
                    "Pagination cursor could not be encoded.",
                )
            })?,
        has_more: page.has_more,
    })
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

#[cfg(test)]
mod tests {
    use super::{ConsentHistoryResult, UserConsent, is_revocable_consent_type};
    use chrono::{Duration, Utc};
    use nvbes_core::pagination::{KeysetCursor, page_from_rows};
    use uuid::Uuid;

    fn paginate(consents: Vec<UserConsent>) -> ConsentHistoryResult {
        let page = page_from_rows(consents, 1, |consent| KeysetCursor {
            created_at: consent.granted_at,
            id: consent.id,
        });

        ConsentHistoryResult {
            consents: page.items,
            next_cursor: page.next_cursor.map(|cursor| cursor.encode().unwrap()),
            has_more: page.has_more,
        }
    }

    #[test]
    fn consent_history_is_paginated_and_stable() {
        let now = Utc::now();
        let consents = vec![
            UserConsent {
                id: Uuid::new_v4(),
                principal_id: Uuid::new_v4(),
                consent_type: "marketing".to_string(),
                document_version: "v1".to_string(),
                ip_address: None,
                granted_at: now - Duration::minutes(2),
                revoked_at: None,
            },
            UserConsent {
                id: Uuid::new_v4(),
                principal_id: Uuid::new_v4(),
                consent_type: "privacy".to_string(),
                document_version: "v1".to_string(),
                ip_address: None,
                granted_at: now - Duration::minutes(1),
                revoked_at: None,
            },
        ];

        let first = paginate(consents);
        assert_eq!(first.consents.len(), 1);
        assert!(first.has_more);
        assert!(first.next_cursor.is_some());
    }

    #[test]
    fn required_legal_agreements_are_not_revocable() {
        assert!(!is_revocable_consent_type("terms_of_service"));
        assert!(!is_revocable_consent_type("data_processing_agreement"));
        assert!(!is_revocable_consent_type("privacy_policy"));
        assert!(is_revocable_consent_type("marketing_emails"));
    }
}
