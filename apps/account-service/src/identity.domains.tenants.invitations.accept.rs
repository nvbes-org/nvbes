use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use super::types::{AcceptIdentityInvitationInput, AcceptIdentityInvitationResponse};
use crate::{domains::auth, http::error::AppError};
use nvbes_core::auth::{normalize_email, token_hash};

pub async fn accept_invitation(
    db: &PgPool,
    auth: &crate::domains::auth::types::AuthContext,
    input: AcceptIdentityInvitationInput,
) -> Result<AcceptIdentityInvitationResponse, AppError> {
    let token_hash_value = token_hash(input.token.trim());
    let current_email = auth::db::fetch_user_record(db, auth.user_id).await?.email;
    let mut tx = db.begin().await?;

    if let Some(response) =
        accept_tenant_invitation(&mut tx, &token_hash_value, &current_email, auth.user_id).await?
    {
        tx.commit().await?;
        return Ok(response);
    }

    if let Some(response) =
        accept_organization_invitation(&mut tx, &token_hash_value, &current_email, auth.user_id)
            .await?
    {
        tx.commit().await?;
        return Ok(response);
    }

    Err(AppError::bad_request(
        "invalid_invitation",
        "Invitation token is invalid.",
    ))
}

async fn accept_tenant_invitation(
    tx: &mut Transaction<'_, Postgres>,
    token_hash_value: &str,
    current_email: &str,
    user_id: Uuid,
) -> Result<Option<AcceptIdentityInvitationResponse>, AppError> {
    let Some(row) = sqlx::query(
        r#"
        SELECT id, tenant_id AS scope_id, email, role::text AS role, status::text AS status, expires_at, accepted_at
        FROM tenant_invitations
        WHERE token_hash = $1
        LIMIT 1
        FOR UPDATE
        "#,
    )
    .bind(token_hash_value)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };

    ensure_invitation_available(&row, current_email)?;
    let scope_id = row.get("scope_id");
    let role: String = row.get("role");
    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, source)
        VALUES ($1, $2, 'human', $3::identity_role, 'active', 'invitation')
        ON CONFLICT (tenant_id, principal_id)
        DO UPDATE SET role = EXCLUDED.role, status = 'active', source = 'invitation', updated_at = NOW()
        "#,
    )
    .bind(scope_id)
    .bind(user_id)
    .bind(&role)
    .execute(&mut **tx)
    .await?;
    let accepted_at = mark_tenant_invitation_accepted(tx, row.get("id")).await?;

    Ok(Some(AcceptIdentityInvitationResponse {
        scope_type: "tenant".to_string(),
        scope_id,
        role,
        accepted_at,
    }))
}

async fn accept_organization_invitation(
    tx: &mut Transaction<'_, Postgres>,
    token_hash_value: &str,
    current_email: &str,
    user_id: Uuid,
) -> Result<Option<AcceptIdentityInvitationResponse>, AppError> {
    let Some(row) = sqlx::query(
        r#"
        SELECT oi.id, oi.organization_id AS scope_id, o.tenant_id, oi.email, oi.role::text AS role,
               oi.status::text AS status, oi.expires_at, oi.accepted_at
        FROM organization_invitations oi
        INNER JOIN organizations o ON o.id = oi.organization_id
        WHERE oi.token_hash = $1
        LIMIT 1
        FOR UPDATE OF oi
        "#,
    )
    .bind(token_hash_value)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(None);
    };

    ensure_invitation_available(&row, current_email)?;
    let scope_id = row.get("scope_id");
    let tenant_id: Uuid = row.get("tenant_id");
    let role: String = row.get("role");
    ensure_tenant_membership(tx, tenant_id, user_id).await?;
    sqlx::query(
        r#"
        INSERT INTO organization_memberships (organization_id, principal_id, role, status, source)
        VALUES ($1, $2, $3::identity_role, 'active', 'invitation')
        ON CONFLICT (organization_id, principal_id)
        DO UPDATE SET role = EXCLUDED.role, status = 'active', source = 'invitation', updated_at = NOW()
        "#,
    )
    .bind(scope_id)
    .bind(user_id)
    .bind(&role)
    .execute(&mut **tx)
    .await?;
    let accepted_at = mark_organization_invitation_accepted(tx, row.get("id")).await?;

    Ok(Some(AcceptIdentityInvitationResponse {
        scope_type: "organization".to_string(),
        scope_id,
        role,
        accepted_at,
    }))
}

async fn ensure_tenant_membership(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, source)
        VALUES ($1, $2, 'human', 'member', 'active', 'invitation')
        ON CONFLICT (tenant_id, principal_id) DO NOTHING
        "#,
    )
    .bind(tenant_id)
    .bind(user_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn ensure_invitation_available(
    row: &sqlx::postgres::PgRow,
    current_email: &str,
) -> Result<(), AppError> {
    let email: String = row.get("email");
    let status: String = row.get("status");
    let expires_at: DateTime<Utc> = row.get("expires_at");
    let accepted_at: Option<DateTime<Utc>> = row.get("accepted_at");

    if normalize_email(&email) != normalize_email(current_email) {
        return Err(AppError::forbidden(
            "invitation_email_mismatch",
            "This invitation was issued to another email address.",
        ));
    }
    if status != "pending" || accepted_at.is_some() || expires_at <= Utc::now() {
        return Err(AppError::bad_request(
            "invitation_unavailable",
            "Invitation is no longer available.",
        ));
    }
    Ok(())
}

async fn mark_tenant_invitation_accepted(
    tx: &mut Transaction<'_, Postgres>,
    invitation_id: Uuid,
) -> Result<DateTime<Utc>, AppError> {
    let accepted_at = Utc::now();
    sqlx::query("UPDATE tenant_invitations SET status = 'accepted', accepted_at = $2, updated_at = $2 WHERE id = $1")
        .bind(invitation_id)
        .bind(accepted_at)
        .execute(&mut **tx)
        .await?;
    Ok(accepted_at)
}

async fn mark_organization_invitation_accepted(
    tx: &mut Transaction<'_, Postgres>,
    invitation_id: Uuid,
) -> Result<DateTime<Utc>, AppError> {
    let accepted_at = Utc::now();
    sqlx::query("UPDATE organization_invitations SET status = 'accepted', accepted_at = $2, updated_at = $2 WHERE id = $1")
        .bind(invitation_id)
        .bind(accepted_at)
        .execute(&mut **tx)
        .await?;
    Ok(accepted_at)
}
