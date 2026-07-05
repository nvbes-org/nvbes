use chrono::{DateTime, NaiveDate, Utc};
use nvbes_core::config::AppConfig;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::domains::auth::{email_verification, password, types::derive_display_name};
use crate::http::error::AppError;

const CURRENT_LEGAL_DOCUMENT_VERSION: &str = "2026-06-26";
const REGISTER_LEGAL_DOCUMENTS: [(&str, &str); 3] = [
    ("terms_of_service", CURRENT_LEGAL_DOCUMENT_VERSION),
    ("privacy_policy", CURRENT_LEGAL_DOCUMENT_VERSION),
    ("data_processing_agreement", CURRENT_LEGAL_DOCUMENT_VERSION),
];

#[expect(
    clippy::too_many_arguments,
    reason = "Account creation keeps registration, workspace, and audit inputs explicit."
)]
pub async fn create_user_account(
    db: &PgPool,
    redis: &nvbes_redis::RedisPool,
    config: &AppConfig,
    email: String,
    firstname: String,
    lastname: String,
    username: String,
    birthdate: Option<NaiveDate>,
    region: Option<String>,
    data_region: Option<String>,
    workspace_name: String,
    password_hash: String,
    verification_token: String,
    ip: Option<String>,
    user_agent: Option<String>,
    legal_documents_accepted: bool,
    marketing_emails_accepted: bool,
) -> Result<(Uuid, Uuid, DateTime<Utc>), AppError> {
    if !legal_documents_accepted {
        return Err(AppError::bad_request(
            "legal_documents_required",
            "Legal documents must be accepted to create an account.",
        ));
    }

    let now = Utc::now();
    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let slug = password::unique_slug(&email);
    let display_name = derive_display_name(
        Some(firstname.as_str()),
        Some(lastname.as_str()),
        Some(&username),
    );

    let mut tx = db.begin().await?;

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'personal', $2, $3, 'active', 'standard', $4, $4)
        "#,
    )
    .bind(tenant_id)
    .bind(&username)
    .bind(slug)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'human', 'active', $3, $4, $4)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(Option::<String>::None)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO users (principal_id, email, firstname, lastname, username, birthdate, region, password_hash, email_verified_at, status, password_last_changed_at, created_at, updated_at, notifications)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, NULL, 'pending_verification', $9, $9, $9, $10)
        "#,
    )
    .bind(principal_id)
    .bind(&email)
    .bind(firstname)
    .bind(lastname)
    .bind(Some(username))
    .bind(birthdate)
    .bind(&region)
    .bind(&password_hash)
    .bind(now)
    .bind(initial_notifications(marketing_emails_accepted))
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO user_email_addresses (
          principal_id,
          email,
          normalized_email,
          is_primary,
          verified_at,
          created_at,
          updated_at
        )
        VALUES ($1, $2, lower($2), TRUE, NULL, $3, $3)
        "#,
    )
    .bind(principal_id)
    .bind(&email)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, status, source, created_at, updated_at)
        VALUES ($1, $2, 'human', 'active', 'manual', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO workspaces (id, tenant_id, organization_id, name, workspace_type, plan_code, trial_ends_at, data_region, jurisdiction, created_at, updated_at)
        VALUES ($1, $2, NULL, $3, 'personal', 'solo_pro', $4, $5::data_region, $6::legal_jurisdiction, $7, $7)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(&workspace_name)
    .bind(now + chrono::Duration::days(14))
    .bind(data_region.as_deref().unwrap_or("eu"))
    .bind(match data_region.as_deref() {
        Some("us") => "ccpa",
        Some("ch") => "nfdap",
        _ => "gdpr",
    })
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO workspace_policies (
          workspace_id,
          member_can_create_share_links,
          require_admin_approval_for_member_share,
          default_share_link_ttl_days,
          max_share_link_ttl_days,
          required_acr,
          created_at,
          updated_at
        )
        VALUES ($1, FALSE, TRUE, 7, 30, 'aal1', $2, $2)
        "#,
    )
    .bind(workspace_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
        VALUES ($1, $2, 'owner', 'active', 'manual', $3, $3)
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .bind(now)
    .execute(&mut *tx)
    .await?;

    record_registration_legal_consents_tx(&mut tx, principal_id, ip.as_deref(), now).await?;

    crate::domains::audit::record_event_tx(
        &mut tx,
        crate::domains::audit::AuditRecordInput {
            tenant_id,
            workspace_id: Some(workspace_id),
            actor_principal_id: Some(principal_id),
            action: "user.registered",
            target_type: "user",
            target_id: Some(principal_id),
            ip: ip.as_deref(),
            user_agent: user_agent.as_deref(),
            metadata: serde_json::json!({"email": email}),
        },
    )
    .await?;

    tx.commit().await?;

    email_verification::issue_verification_email_tx(
        redis,
        config,
        principal_id,
        &email,
        &display_name,
        &verification_token,
    )
    .await?;

    Ok((principal_id, workspace_id, now))
}

fn initial_notifications(marketing_emails_accepted: bool) -> serde_json::Value {
    serde_json::json!({
        "email": true,
        "push": true,
        "in_app": true,
        "marketing_email": marketing_emails_accepted,
    })
}

async fn record_registration_legal_consents_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    ip: Option<&str>,
    granted_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    let masked_ip = ip.and_then(crate::domains::legal::db::anonymize_ip_str);

    for (consent_type, document_version) in REGISTER_LEGAL_DOCUMENTS {
        sqlx::query(
            r#"
            INSERT INTO user_consents (
              principal_id, consent_type, document_version, ip_address, granted_at, revoked_at
            )
            VALUES ($1, $2, $3, $4::inet, $5, NULL)
            ON CONFLICT (principal_id, document_version, consent_type)
            DO UPDATE SET granted_at = EXCLUDED.granted_at,
                          revoked_at = NULL,
                          ip_address = EXCLUDED.ip_address
            "#,
        )
        .bind(principal_id)
        .bind(consent_type)
        .bind(document_version)
        .bind(masked_ip.as_deref())
        .bind(granted_at)
        .execute(&mut **tx)
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{REGISTER_LEGAL_DOCUMENTS, initial_notifications};

    #[test]
    fn registration_legal_documents_are_recorded_per_document() {
        assert_eq!(
            REGISTER_LEGAL_DOCUMENTS.map(|(consent_type, _)| consent_type),
            [
                "terms_of_service",
                "privacy_policy",
                "data_processing_agreement"
            ]
        );
    }

    #[test]
    fn initial_notifications_keep_marketing_opt_in_separate() {
        let notifications = initial_notifications(true);

        assert_eq!(notifications["email"], true);
        assert_eq!(notifications["push"], true);
        assert_eq!(notifications["in_app"], true);
        assert_eq!(notifications["marketing_email"], true);
    }
}
