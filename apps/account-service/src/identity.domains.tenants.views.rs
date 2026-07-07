use sqlx::Row;

use super::types::{IdentityMembershipView, OrganizationView, TenantView};

pub fn tenant_view(row: sqlx::postgres::PgRow) -> TenantView {
    TenantView {
        id: row.get("id"),
        kind: row.get("kind"),
        name: row.get("name"),
        slug: row.get("slug"),
        status: row.get("status"),
        security_tier: row.get("security_tier"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub fn organization_view(row: sqlx::postgres::PgRow) -> OrganizationView {
    OrganizationView {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        parent_organization_id: row.get("parent_organization_id"),
        name: row.get("name"),
        slug: row.get("slug"),
        status: row.get("status"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

pub fn membership_view(scope_type: &str, row: sqlx::postgres::PgRow) -> IdentityMembershipView {
    IdentityMembershipView {
        scope_type: scope_type.to_string(),
        scope_id: row.get("scope_id"),
        principal_id: row.get("principal_id"),
        role: row.get("role"),
        status: row.get("status"),
        source: row.get("source"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}
