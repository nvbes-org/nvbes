use super::service::ConsentRequirementInput;
use crate::http::error::AppError;
use sqlx::PgPool;

pub async fn ensure_consent(db: &PgPool, input: ConsentRequirementInput) -> Result<(), AppError> {
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
    .fetch_optional(db)
    .await?;

    if existing.is_some() {
        return Ok(());
    }

    if input
        .consent_action
        .as_deref()
        .map(str::trim)
        .map(str::to_lowercase)
        .as_deref()
        != Some("approve")
    {
        return Err(AppError::forbidden(
            "consent_required",
            "Explicit consent is required for this OAuth client and scope.",
        ));
    }

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
          resource_indicators
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
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
    .execute(db)
    .await?;

    Ok(())
}
