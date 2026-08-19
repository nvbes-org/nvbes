use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domains::{
    auth::types::{AuthContext, AuthPrincipalKind},
    authz::{WorkspaceAccess, WorkspaceRole},
    public_api::types::PublicApiContext,
};

pub(crate) fn test_pool() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .or_else(|_| std::env::var("NVBES_DATABASE_URL"))
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/nvbes".to_string());
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(std::time::Duration::from_millis(500))
        .connect_lazy(&url)
        .expect("valid pool")
}

pub(crate) async fn db_supports_public_api_geo_schema(pool: &PgPool) -> bool {
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

pub(crate) async fn seed_workspace(pool: &PgPool, key: &str) -> (Uuid, Uuid, Uuid) {
    let user_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let tenant_id = Uuid::new_v4();
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO users (
          id, email, email_verified_at, display_name, status, mfa_enabled, created_at, updated_at
        )
        VALUES ($1, $2, NOW(), 'Drive DB Test', 'active', false, $3, $3)
        "#,
    )
    .bind(user_id)
    .bind(format!("{key}@example.com"))
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
        VALUES ($1, 'team', 'Drive DB Test Tenant', $2, 'active', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("drive-db-test-{tenant_id}"))
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
        VALUES ($1, 'team', 'Drive DB Test Workspace', $2, $2, $3, $4, $4, $5)
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

pub(crate) async fn seed_vpn_range(pool: &PgPool, key: &str) {
    seed_geo_range(pool, key, "8.8.4.0/24", "vpn", 95, &["vpn", "anonymous"]).await;
}

pub(crate) async fn seed_geo_range(
    pool: &PgPool,
    key: &str,
    network: &str,
    network_kind: &str,
    risk_score: i16,
    risk_labels: &[&str],
) {
    let labels: Vec<String> = risk_labels
        .iter()
        .map(|label| (*label).to_string())
        .collect();
    sqlx::query(
        r#"
        INSERT INTO geo_personal_ip_ranges (
          network, country_code, source_reference, note, network_kind, risk_score, risk_labels
        )
        VALUES ($1::cidr, 'FR', $2, 'drive db test reputation range', $3, $4, $5)
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
    .bind(network)
    .bind(key)
    .bind(network_kind)
    .bind(risk_score)
    .bind(&labels)
    .execute(pool)
    .await
    .expect("geo range insert should succeed");
}

pub(crate) fn public_api_context(
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

pub(crate) fn workspace_access(
    principal_id: Uuid,
    workspace_id: Uuid,
    tenant_id: Uuid,
) -> WorkspaceAccess {
    let auth = AuthContext {
        principal_id,
        principal_kind: AuthPrincipalKind::User,
        user_id: principal_id,
        email_verified_at: Some(Utc::now()),
        session_id: Uuid::new_v4(),
        tenant_id: Some(tenant_id),
        organization_id: None,
        workspace_id: Some(workspace_id),
        scope: "drive.audit.read".to_string(),
        role: Some("owner".to_string()),
        amr: vec!["pwd".to_string()],
        actor: None,
        acr: Some("aal1".to_string()),
        auth_time: Some(Utc::now()),
    };
    WorkspaceAccess::new(
        auth,
        workspace_id,
        Some(tenant_id),
        None,
        WorkspaceRole::Owner,
        nvbes_tenancy::WorkspacePolicy::member_share_links_enabled(true),
    )
}
