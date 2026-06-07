use nvbes_core::auth::normalize_email;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    domains::federation::{
        types::{
            CreateLinkedIdentityInput, LinkedIdentitiesResponse, LinkedIdentityRecord,
            LinkedIdentityResponse, LinkedIdentityView, PrincipalRecord,
        },
        validation::normalize_federated_provider_type,
    },
    http::error::AppError,
};

pub async fn list_linked_identities(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<LinkedIdentitiesResponse, AppError> {
    let rows = sqlx::query_as::<_, LinkedIdentityRecord>(
        r#"
        SELECT
          ui.id,
          ui.principal_id,
          ui.provider_type::text AS provider_type,
          ui.provider_id,
          ui.subject,
          ui.email,
          ui.email_verified,
          ui.created_at
        FROM user_identities ui
        INNER JOIN principals p ON p.id = ui.principal_id
        WHERE p.tenant_id = $1
        ORDER BY ui.created_at DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(LinkedIdentitiesResponse {
        identities: rows
            .into_iter()
            .map(LinkedIdentityRecord::into_view)
            .collect(),
    })
}

pub async fn link_identity(
    db: &PgPool,
    tenant_id: Uuid,
    input: CreateLinkedIdentityInput,
) -> Result<LinkedIdentityResponse, AppError> {
    let principal = fetch_principal_for_tenant(db, tenant_id, input.principal_id).await?;
    let provider_type = normalize_federated_provider_type(&input.provider_type)?;
    let email = input.email.as_deref().map(normalize_email);

    if let Some(email) = &email {
        let identity_owner = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT principal_id
            FROM users
            WHERE lower(email) = $1
            LIMIT 1
            "#,
        )
        .bind(email)
        .fetch_optional(db)
        .await?;

        if let Some(existing_principal_id) = identity_owner {
            if existing_principal_id != input.principal_id {
                return Err(AppError::conflict(
                    crate::domains::federation::contract::IDENTITY_CONFLICT,
                    "The email is already linked to another principal.",
                ));
            }
        }
    }

    let existing = sqlx::query_as::<_, LinkedIdentityRecord>(
        r#"
        SELECT
          id,
          principal_id,
          provider_type::text AS provider_type,
          provider_id,
          subject,
          email,
          email_verified,
          created_at
        FROM user_identities
        WHERE provider_type = $1::identity_provider_type
          AND provider_id = $2
          AND subject = $3
        LIMIT 1
        "#,
    )
    .bind(&provider_type)
    .bind(input.provider_id.trim())
    .bind(input.subject.trim())
    .fetch_optional(db)
    .await?;

    if let Some(existing) = existing {
        if existing.principal_id != input.principal_id {
            return Err(AppError::conflict(
                crate::domains::federation::contract::IDENTITY_CONFLICT,
                "This external identity is already linked to another principal.",
            ));
        }

        return Ok(LinkedIdentityResponse {
            identity: existing.into_view(),
        });
    }

    let row = sqlx::query_as::<_, LinkedIdentityRecord>(
        r#"
        INSERT INTO user_identities (
          principal_id,
          provider_type,
          provider_id,
          subject,
          email,
          email_verified
        )
        VALUES ($1, $2::identity_provider_type, $3, $4, $5, $6)
        RETURNING
          id,
          principal_id,
          provider_type::text AS provider_type,
          provider_id,
          subject,
          email,
          email_verified,
          created_at
        "#,
    )
    .bind(principal.id)
    .bind(provider_type)
    .bind(input.provider_id.trim())
    .bind(input.subject.trim())
    .bind(email)
    .bind(input.email_verified)
    .fetch_one(db)
    .await?;

    Ok(LinkedIdentityResponse {
        identity: row.into_view(),
    })
}

pub async fn unlink_identity(
    db: &PgPool,
    tenant_id: Uuid,
    identity_id: Uuid,
) -> Result<(), AppError> {
    let deleted = sqlx::query(
        r#"
        DELETE FROM user_identities
        WHERE id = $1
          AND principal_id IN (
            SELECT id
            FROM principals
            WHERE tenant_id = $2
          )
        "#,
    )
    .bind(identity_id)
    .bind(tenant_id)
    .execute(db)
    .await?
    .rows_affected();

    if deleted == 0 {
        return Err(AppError::not_found(
            "identity_not_found",
            "Linked identity not found.",
        ));
    }

    Ok(())
}

pub async fn link_or_reuse_identity(
    db: &PgPool,
    principal_id: Uuid,
    provider_type: &str,
    provider_id: Uuid,
    email: &str,
    subject: &str,
    email_verified: bool,
) -> Result<LinkedIdentityView, AppError> {
    if let Some(existing) = sqlx::query_as::<_, LinkedIdentityRecord>(
        r#"
        SELECT
          id,
          principal_id,
          provider_type::text AS provider_type,
          provider_id,
          subject,
          email,
          email_verified,
          created_at
        FROM user_identities
        WHERE provider_type = $1::identity_provider_type
          AND provider_id = $2
          AND subject = $3
        LIMIT 1
        "#,
    )
    .bind(provider_type)
    .bind(provider_id.to_string())
    .bind(subject.trim())
    .fetch_optional(db)
    .await?
    {
        if existing.principal_id != principal_id {
            return Err(AppError::conflict(
                crate::domains::federation::contract::IDENTITY_CONFLICT,
                "This external identity is already linked to another principal.",
            ));
        }

        return Ok(existing.into_view());
    }

    let row = sqlx::query_as::<_, LinkedIdentityRecord>(
        r#"
        INSERT INTO user_identities (
          principal_id,
          provider_type,
          provider_id,
          subject,
          email,
          email_verified
        )
        VALUES ($1, $2::identity_provider_type, $3, $4, $5, $6)
        RETURNING
          id,
          principal_id,
          provider_type::text AS provider_type,
          provider_id,
          subject,
          email,
          email_verified,
          created_at
        "#,
    )
    .bind(principal_id)
    .bind(provider_type)
    .bind(provider_id.to_string())
    .bind(subject.trim())
    .bind(Some(email.to_string()))
    .bind(email_verified)
    .fetch_one(db)
    .await?;

    Ok(row.into_view())
}

pub async fn fetch_principal_for_tenant(
    db: &PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
) -> Result<PrincipalRecord, AppError> {
    let row = sqlx::query_as::<_, PrincipalRecord>(
        r#"
        SELECT
          id,
          tenant_id,
          principal_kind::text AS principal_kind,
          status,
          display_name,
          created_at,
          updated_at
        FROM principals
        WHERE id = $1
          AND tenant_id = $2
        LIMIT 1
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .fetch_optional(db)
    .await?;

    row.ok_or_else(|| AppError::not_found("principal_not_found", "Principal not found."))
}
