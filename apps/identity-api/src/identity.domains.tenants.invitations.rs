use chrono::{Duration, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{
    types::{
        IdentityInvitationResponse, IdentityInvitationView, InviteIdentityMemberInput,
    },
    validation::{parse_identity_role_input, role_as_db},
};
use crate::http::error::AppError;
use nvbes_core::{
    auth::{generate_token, log_dev_token, normalize_email, token_hash, validate_email},
    config::AppConfig,
};

pub async fn create_invitation(
    db: &PgPool,
    config: &AppConfig,
    scope_type: &str,
    scope_id: Uuid,
    actor_id: Uuid,
    input: InviteIdentityMemberInput,
) -> Result<IdentityInvitationResponse, AppError> {
    let email = normalize_email(&input.email);
    validate_email(&email)?;
    let role = role_as_db(parse_identity_role_input(&input.role)?);
    let token = generate_token("b2b");
    log_dev_token(&token, &config.environment, "identity_invitation");
    let expires_at = Utc::now() + Duration::days(input.expires_in_days.unwrap_or(7).clamp(1, 30));
    let token_hash = token_hash(&token);
    ensure_no_pending_invitation(db, scope_type, scope_id, &email).await?;

    let row = match scope_type {
        "tenant" => {
            sqlx::query(
                r#"
                INSERT INTO tenant_invitations (tenant_id, email, role, invited_by, token_hash, expires_at)
                VALUES ($1, $2, $3::identity_role, $4, $5, $6)
                RETURNING id, tenant_id AS scope_id, email, role::text AS role, status::text AS status, expires_at, accepted_at, revoked_at, created_at
                "#,
            )
            .bind(scope_id)
            .bind(&email)
            .bind(role)
            .bind(actor_id)
            .bind(&token_hash)
            .bind(expires_at)
            .fetch_one(db)
            .await?
        }
        "organization" => {
            sqlx::query(
                r#"
                INSERT INTO organization_invitations (organization_id, email, role, invited_by, token_hash, expires_at)
                VALUES ($1, $2, $3::identity_role, $4, $5, $6)
                RETURNING id, organization_id AS scope_id, email, role::text AS role, status::text AS status, expires_at, accepted_at, revoked_at, created_at
                "#,
            )
            .bind(scope_id)
            .bind(&email)
            .bind(role)
            .bind(actor_id)
            .bind(&token_hash)
            .bind(expires_at)
            .fetch_one(db)
            .await?
        }
        _ => {
            return Err(AppError::bad_request(
                "validation_failed",
                "Unsupported invitation scope.",
            ));
        }
    };

    Ok(IdentityInvitationResponse {
        invitation: IdentityInvitationView {
            id: row.get("id"),
            scope_type: scope_type.to_string(),
            scope_id: row.get("scope_id"),
            email: row.get("email"),
            role: row.get("role"),
            status: row.get("status"),
            expires_at: row.get("expires_at"),
            accepted_at: row.get("accepted_at"),
            revoked_at: row.get("revoked_at"),
            created_at: row.get("created_at"),
        },
        invitation_token: token,
    })
}

async fn ensure_no_pending_invitation(
    db: &PgPool,
    scope_type: &str,
    scope_id: Uuid,
    email: &str,
) -> Result<(), AppError> {
    let pending = match scope_type {
        "tenant" => {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM tenant_invitations
                WHERE tenant_id = $1
                  AND lower(email) = $2
                  AND status = 'pending'
                  AND expires_at > NOW()
                "#,
            )
            .bind(scope_id)
            .bind(email)
            .fetch_one(db)
            .await?
        }
        "organization" => {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM organization_invitations
                WHERE organization_id = $1
                  AND lower(email) = $2
                  AND status = 'pending'
                  AND expires_at > NOW()
                "#,
            )
            .bind(scope_id)
            .bind(email)
            .fetch_one(db)
            .await?
        }
        _ => 0,
    };

    if pending > 0 {
        return Err(AppError::conflict(
            "invitation_pending",
            "A pending invitation already exists for this email.",
        ));
    }

    Ok(())
}
