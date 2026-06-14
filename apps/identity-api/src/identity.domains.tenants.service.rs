use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{
    invitations::create_invitation,
    types::*,
    validation::{
        parse_identity_role_input, require_name, require_slug, require_status, require_token_value,
        role_as_db,
    },
    views::{membership_view, organization_view, tenant_view},
};
use crate::http::error::AppError;
use nvbes_core::config::AppConfig;

pub async fn get_tenant(db: &PgPool, tenant_id: Uuid) -> Result<TenantResponse, AppError> {
    let row = sqlx::query(
        r#"
        SELECT id, kind::text AS kind, name, slug, status::text AS status, security_tier, created_at, updated_at
        FROM tenants
        WHERE id = $1
        "#,
    )
    .bind(tenant_id)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("tenant_not_found", "Tenant not found."))?;

    Ok(TenantResponse {
        tenant: tenant_view(row),
    })
}

pub async fn update_tenant(
    db: &PgPool,
    tenant_id: Uuid,
    input: UpdateTenantInput,
) -> Result<TenantResponse, AppError> {
    let name = input.name.map(|value| require_name(&value)).transpose()?;
    let security_tier = input
        .security_tier
        .map(|value| require_token_value(&value, "security_tier"))
        .transpose()?;
    let row = sqlx::query(
        r#"
        UPDATE tenants
        SET name = COALESCE($2, name),
            security_tier = COALESCE($3, security_tier),
            updated_at = NOW()
        WHERE id = $1
        RETURNING id, kind::text AS kind, name, slug, status::text AS status, security_tier, created_at, updated_at
        "#,
    )
    .bind(tenant_id)
    .bind(name)
    .bind(security_tier)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("tenant_not_found", "Tenant not found."))?;

    Ok(TenantResponse {
        tenant: tenant_view(row),
    })
}

pub async fn list_organizations(
    db: &PgPool,
    tenant_id: Uuid,
) -> Result<OrganizationsResponse, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT id, tenant_id, parent_organization_id, name, slug, status::text AS status, created_at, updated_at
        FROM organizations
        WHERE tenant_id = $1
        ORDER BY name ASC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(db)
    .await?;

    Ok(OrganizationsResponse {
        organizations: rows.into_iter().map(organization_view).collect(),
    })
}

pub async fn create_organization(
    db: &PgPool,
    tenant_id: Uuid,
    actor_id: Uuid,
    input: CreateOrganizationInput,
) -> Result<OrganizationResponse, AppError> {
    let name = require_name(&input.name)?;
    let slug = require_slug(&input.slug)?;
    let mut tx = db.begin().await?;
    let row = sqlx::query(
        r#"
        INSERT INTO organizations (tenant_id, parent_organization_id, name, slug)
        VALUES ($1, $2, $3, $4)
        RETURNING id, tenant_id, parent_organization_id, name, slug, status::text AS status, created_at, updated_at
        "#,
    )
    .bind(tenant_id)
    .bind(input.parent_organization_id)
    .bind(name)
    .bind(slug)
    .fetch_one(&mut *tx)
    .await?;

    let organization_id: Uuid = row.get("id");
    sqlx::query(
        r#"
        INSERT INTO organization_memberships (organization_id, principal_id, role, status, source)
        VALUES ($1, $2, 'owner', 'active', 'manual')
        ON CONFLICT (organization_id, principal_id)
        DO UPDATE SET role = 'owner', status = 'active', updated_at = NOW()
        "#,
    )
    .bind(organization_id)
    .bind(actor_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(OrganizationResponse {
        organization: organization_view(row),
    })
}

pub async fn update_organization(
    db: &PgPool,
    tenant_id: Uuid,
    organization_id: Uuid,
    input: UpdateOrganizationInput,
) -> Result<OrganizationResponse, AppError> {
    let name = input.name.map(|value| require_name(&value)).transpose()?;
    let status = input
        .status
        .map(|value| require_status(&value))
        .transpose()?;
    let row = sqlx::query(
        r#"
        UPDATE organizations
        SET name = COALESCE($3, name),
            status = COALESCE($4::organization_status, status),
            updated_at = NOW()
        WHERE tenant_id = $1 AND id = $2
        RETURNING id, tenant_id, parent_organization_id, name, slug, status::text AS status, created_at, updated_at
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(name)
    .bind(status)
    .fetch_optional(db)
    .await?
    .ok_or_else(|| AppError::not_found("organization_not_found", "Organization not found."))?;

    Ok(OrganizationResponse {
        organization: organization_view(row),
    })
}

pub async fn ensure_organization_in_tenant(
    db: &PgPool,
    tenant_id: Uuid,
    organization_id: Uuid,
) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM organizations WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(organization_id)
    .fetch_one(db)
    .await?;

    if exists == 0 {
        return Err(AppError::not_found(
            "organization_not_found",
            "Organization not found.",
        ));
    }

    Ok(())
}

pub async fn upsert_tenant_membership(
    db: &PgPool,
    tenant_id: Uuid,
    principal_id: Uuid,
    input: UpsertIdentityMembershipInput,
) -> Result<IdentityMembershipResponse, AppError> {
    let role = role_as_db(parse_identity_role_input(&input.role)?);
    let row = sqlx::query(
        r#"
        INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, source)
        VALUES ($1, $2, 'human', $3::identity_role, 'active', 'manual')
        ON CONFLICT (tenant_id, principal_id)
        DO UPDATE SET role = EXCLUDED.role, status = 'active', updated_at = NOW()
        RETURNING tenant_id AS scope_id, principal_id, role::text AS role, status::text AS status, source::text AS source, created_at, updated_at
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .bind(role)
    .fetch_one(db)
    .await?;

    Ok(IdentityMembershipResponse {
        membership: membership_view("tenant", row),
    })
}

pub async fn upsert_organization_membership(
    db: &PgPool,
    organization_id: Uuid,
    principal_id: Uuid,
    input: UpsertIdentityMembershipInput,
) -> Result<IdentityMembershipResponse, AppError> {
    let role = role_as_db(parse_identity_role_input(&input.role)?);
    let row = sqlx::query(
        r#"
        INSERT INTO organization_memberships (organization_id, principal_id, role, status, source)
        VALUES ($1, $2, $3::identity_role, 'active', 'manual')
        ON CONFLICT (organization_id, principal_id)
        DO UPDATE SET role = EXCLUDED.role, status = 'active', updated_at = NOW()
        RETURNING organization_id AS scope_id, principal_id, role::text AS role, status::text AS status, source::text AS source, created_at, updated_at
        "#,
    )
    .bind(organization_id)
    .bind(principal_id)
    .bind(role)
    .fetch_one(db)
    .await?;

    Ok(IdentityMembershipResponse {
        membership: membership_view("organization", row),
    })
}

pub async fn invite_tenant_member(
    db: &PgPool,
    config: &AppConfig,
    tenant_id: Uuid,
    actor_id: Uuid,
    input: InviteIdentityMemberInput,
) -> Result<IdentityInvitationResponse, AppError> {
    create_invitation(db, config, "tenant", tenant_id, actor_id, input).await
}

pub async fn invite_organization_member(
    db: &PgPool,
    config: &AppConfig,
    organization_id: Uuid,
    actor_id: Uuid,
    input: InviteIdentityMemberInput,
) -> Result<IdentityInvitationResponse, AppError> {
    create_invitation(db, config, "organization", organization_id, actor_id, input).await
}

pub async fn accept_identity_invitation(
    db: &PgPool,
    auth: &crate::domains::auth::types::AuthContext,
    input: AcceptIdentityInvitationInput,
) -> Result<AcceptIdentityInvitationResponse, AppError> {
    super::invitations_accept::accept_invitation(db, auth, input).await
}
