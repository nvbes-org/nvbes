use super::service::ConsentRequirementInput;
use super::system_clients::is_system_client;
use crate::http::error::AppError;
use sqlx::{PgPool, Row};

pub fn evaluate_consent_action(
    consent_action: Option<&str>,
    requires_admin_consent: bool,
    is_tenant_admin: bool,
) -> Result<(), AppError> {
    let approved = consent_action
        .map(str::trim)
        .map(str::to_lowercase)
        .as_deref()
        == Some("approve");

    if !approved {
        return Err(AppError::forbidden(
            "consent_required",
            "Explicit consent is required for this OAuth client and scope.",
        ));
    }

    if requires_admin_consent && !is_tenant_admin {
        return Err(AppError::forbidden(
            "admin_consent_required",
            "Tenant administrator consent is required for this OAuth client and scope.",
        ));
    }

    Ok(())
}

pub async fn ensure_consent(db: &PgPool, input: ConsentRequirementInput) -> Result<(), AppError> {
    let requires_admin_consent = requires_admin_consent(db, input.client_id, &input.scope).await?;
    let existing = sqlx::query(
        r#"
        SELECT id
        FROM oauth_consents
        WHERE principal_id = $1
          AND client_id = $2
          AND tenant_id IS NOT DISTINCT FROM $3
          AND organization_id IS NOT DISTINCT FROM $4
          AND workspace_id IS NOT DISTINCT FROM $5
          AND scope = $6
          AND audience IS NOT DISTINCT FROM $7
          AND resource_indicators = $8
          AND ($9 = FALSE OR granted_by_admin = TRUE)
          AND revoked_at IS NULL
          AND (expires_at IS NULL OR expires_at > NOW())
        LIMIT 1
        "#,
    )
    .bind(input.user_id)
    .bind(input.client_id)
    .bind(input.tenant_id)
    .bind(input.organization_id)
    .bind(input.workspace_id)
    .bind(&input.scope)
    .bind(&input.audience)
    .bind(&input.resource_indicators)
    .bind(requires_admin_consent)
    .fetch_optional(db)
    .await?;

    if existing.is_some() {
        return Ok(());
    }

    let is_tenant_admin = if requires_admin_consent {
        tenant_admin_can_grant_consent(db, input.tenant_id, input.user_id).await?
    } else {
        false
    };
    evaluate_consent_action(
        input.consent_action.as_deref(),
        requires_admin_consent,
        is_tenant_admin,
    )?;

    sqlx::query(
        r#"
        INSERT INTO oauth_consents (
          principal_id,
          client_id,
          tenant_id,
          organization_id,
          workspace_id,
          scope,
          audience,
          resource_indicators,
          granted_by_admin
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        "#,
    )
    .bind(input.user_id)
    .bind(input.client_id)
    .bind(input.tenant_id)
    .bind(input.organization_id)
    .bind(input.workspace_id)
    .bind(&input.scope)
    .bind(&input.audience)
    .bind(&input.resource_indicators)
    .bind(is_tenant_admin)
    .execute(db)
    .await?;

    Ok(())
}

async fn requires_admin_consent(
    db: &PgPool,
    client_id: uuid::Uuid,
    scopes: &[String],
) -> Result<bool, AppError> {
    let row = sqlx::query(
        r#"
        SELECT
          c.client_id,
          COALESCE(c.requires_admin_consent, FALSE)
          OR EXISTS (
            SELECT 1
            FROM oauth_scope_metadata scope
            WHERE scope.scope = ANY($2)
              AND scope.requires_admin_consent = TRUE
          ) AS requires_admin_consent
        FROM oauth_clients c
        WHERE c.id = $1
        "#,
    )
    .bind(client_id)
    .bind(scopes)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("client_not_found", "The OAuth client was not found."))?;

    let client_id: String = row.get("client_id");
    if is_system_client(&client_id) {
        return Ok(false);
    }

    Ok(row.get("requires_admin_consent"))
}

async fn tenant_admin_can_grant_consent(
    db: &PgPool,
    tenant_id: Option<uuid::Uuid>,
    principal_id: uuid::Uuid,
) -> Result<bool, AppError> {
    let Some(tenant_id) = tenant_id else {
        return Ok(false);
    };

    let role = sqlx::query_scalar::<_, String>(
        r#"
        SELECT role::text
        FROM tenant_memberships
        WHERE tenant_id = $1
          AND principal_id = $2
          AND status = 'active'
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .fetch_optional(db)
    .await?;

    Ok(role
        .as_deref()
        .is_some_and(|role| matches!(role, "owner" | "admin" | "security_admin")))
}

#[cfg(test)]
mod tests {
    use super::evaluate_consent_action;

    #[test]
    fn evaluate_consent_action_requires_admin_for_sensitive_oauth_clients() {
        let err = evaluate_consent_action(Some("approve"), true, false)
            .expect_err("non-admin consent should be rejected");

        assert_eq!(err.code, "admin_consent_required");
    }

    #[test]
    fn evaluate_consent_action_allows_admin_approval_for_sensitive_oauth_clients() {
        evaluate_consent_action(Some("approve"), true, true)
            .expect("tenant admin approval should be accepted");
    }
}
