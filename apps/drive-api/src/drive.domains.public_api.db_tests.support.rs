use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use super::super::types::PublicApiContext;

pub(super) fn test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string());
    PgPool::connect_lazy(&url).expect("valid pool")
}

pub(super) async fn db_supports_public_api_geo_schema(pool: &PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM information_schema.columns
          WHERE table_name = 'api_request_logs'
            AND column_name = 'geo_risk_labels'
        )
        AND EXISTS (
          SELECT 1
          FROM information_schema.tables
          WHERE table_name = 'public_api_network_allowlist'
        )
        "#,
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false)
}

pub(super) async fn seed_workspace(pool: &PgPool, key: &str) -> (Uuid, Uuid, Uuid) {
    let user_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let now = Utc::now();
    let email = format!("{key}@example.com");

    sqlx::query(
        r#"
        INSERT INTO users (
          id, email, email_verified_at, display_name, status, mfa_enabled, created_at, updated_at
        )
        VALUES ($1, $2, NOW(), 'Drive Public API Test', 'active', false, $3, $3)
        "#,
    )
    .bind(user_id)
    .bind(email)
    .bind(now)
    .execute(pool)
    .await
    .expect("user insert should succeed");

    let plan_id: Uuid = sqlx::query_scalar("SELECT id FROM plans WHERE code = 'team_plus' LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("team_plus plan should exist");

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, created_at, updated_at)
        VALUES ($1, 'team', 'Drive Public API Tenant', $2, 'active', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("drive-public-api-{tenant_id}"))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id, workspace_type, name, owner_user_id, owner_principal_id,
          plan_id, created_at, updated_at, tenant_id
        )
        VALUES ($1, 'team', 'Drive Public API Workspace', $2, $2, $3, $4, $4, $5)
        "#,
    )
    .bind(workspace_id)
    .bind(user_id)
    .bind(plan_id)
    .bind(now)
    .bind(tenant_id)
    .execute(pool)
    .await
    .expect("workspace insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspace_policies (
          workspace_id, member_can_create_share_links, require_admin_approval_for_member_share,
          default_share_link_ttl_days, max_share_link_ttl_days, updated_at
        )
        VALUES ($1, false, true, 7, 30, $2)
        "#,
    )
    .bind(workspace_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("workspace policy insert should succeed");

    (user_id, workspace_id, tenant_id)
}

pub(super) async fn seed_vpn_range(pool: &PgPool, key: &str) {
    sqlx::query(
        r#"
        INSERT INTO geo_personal_ip_ranges (
          network, country_code, source_reference, note, network_kind, risk_score, risk_labels
        )
        VALUES ('8.8.4.0/24', 'FR', $1, 'public api test vpn range', 'vpn', 95, ARRAY['vpn', 'anonymous'])
        ON CONFLICT (network) DO UPDATE SET
          country_code = EXCLUDED.country_code,
          source_reference = EXCLUDED.source_reference,
          network_kind = EXCLUDED.network_kind,
          risk_score = EXCLUDED.risk_score,
          risk_labels = EXCLUDED.risk_labels,
          enabled = TRUE,
          updated_at = NOW()
        "#,
    )
    .bind(key)
    .execute(pool)
    .await
    .expect("vpn range insert should succeed");
}

pub(super) fn context(
    workspace_id: Uuid,
    principal_id: Uuid,
    tenant_id: Uuid,
    request_id: String,
) -> PublicApiContext {
    PublicApiContext {
        api_key_id: None,
        workspace_id,
        created_by: Some(principal_id),
        created_by_principal_id: principal_id,
        tenant_id: Some(tenant_id),
        organization_id: None,
        role: Some("owner".to_string()),
        key_prefix: "nvbes_test".to_string(),
        scopes: vec!["files:read".to_string()],
        plan_code: "team_plus".to_string(),
        request_id,
        m2m_client_id: None,
    }
}
