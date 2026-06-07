use chrono::Utc;
use sqlx::postgres::PgPool;
use uuid::Uuid;

pub(crate) async fn seed_machine_workspace_context(
    pool: &PgPool,
) -> (Uuid, Uuid, String, String, Uuid, String) {
    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let owner_user_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();
    let client_id = format!("gxoc_drive_e2e_{}", Uuid::new_v4().simple());
    let client_secret = format!("gxo_drive_e2e_{}", Uuid::new_v4().simple());
    let client_secret_hash = nvbes_identity_api::domains::oauth::hash_client_secret(&client_secret)
        .expect("secret should hash");
    let owner_email = format!("drive-auth-e2e-{}@example.com", Uuid::new_v4());
    let now = Utc::now();

    let plan_id: Uuid = sqlx::query_scalar("SELECT id FROM plans ORDER BY created_at ASC LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("plan should exist");

    sqlx::query(
        r#"
        INSERT INTO users (
          id,
          email,
          email_verified_at,
          display_name,
          status,
          mfa_enabled,
          created_at,
          updated_at
        )
        VALUES ($1, $2, NOW(), 'Drive E2E Owner', 'active', false, $3, $3)
        "#,
    )
    .bind(owner_user_id)
    .bind(&owner_email)
    .bind(now)
    .execute(pool)
    .await
    .expect("owner user insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'team', 'Drive E2E Tenant', $2, 'active', 'standard', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("drive-auth-e2e-{tenant_id}"))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspaces (
          id,
          tenant_id,
          workspace_type,
          name,
          owner_user_id,
          owner_principal_id,
          plan_id,
          plan_code,
          created_at,
          updated_at
        )
        VALUES ($1, $2, 'team', 'Drive E2E Workspace', $3, $3, $4, 'team_plus', $5, $5)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .bind(owner_user_id)
    .bind(plan_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("workspace insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspace_policies (
          workspace_id,
          member_can_create_share_links,
          require_admin_approval_for_member_share,
          default_share_link_ttl_days,
          max_share_link_ttl_days,
          updated_at
        )
        VALUES ($1, false, true, 7, 30, $2)
        ON CONFLICT (workspace_id) DO NOTHING
        "#,
    )
    .bind(workspace_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("workspace policy insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
        VALUES ($1, $2, 'service_account', 'active', 'Drive E2E Robot', $3, $3)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("principal insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO service_accounts (
          principal_id,
          tenant_id,
          workspace_id,
          created_by_principal_id,
          name,
          description,
          auth_method,
          client_id,
          last_rotated_at,
          created_at,
          updated_at
        )
        VALUES ($1, $2, $3, NULL, 'Drive E2E Robot', 'drive e2e machine', 'oauth_client_credentials', $4, $5, $5, $5)
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(&client_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("service account insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
          id,
          client_id,
          client_secret_hash,
          name,
          redirect_uris,
          tenant_id,
          owner_scope_type,
          owner_scope_id,
          client_type,
          revoked_at,
          created_at,
          updated_at
        )
        VALUES ($1, $2, $3, 'Drive E2E Client', ARRAY['https://example.com/callback'], $4, 'workspace', $5, 'service', NULL, $6, $6)
        "#,
    )
    .bind(client_uuid)
    .bind(&client_id)
    .bind(&client_secret_hash)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("oauth client insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO oauth_client_policies (
          client_id,
          scope_type,
          scope_id,
          allowed_scopes,
          allowed_audiences,
          allowed_resources,
          required_acr,
          status,
          created_at
        )
        VALUES ($1, 'workspace', $2, ARRAY['drive.files.read', 'drive.workspace.read'], ARRAY['nvbes-drive-api'], ARRAY['drive'], 'aal1', 'active', $3)
        "#,
    )
    .bind(client_uuid)
    .bind(workspace_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("oauth policy insert should succeed");

    (
        tenant_id,
        workspace_id,
        client_id,
        client_secret,
        principal_id,
        owner_email,
    )
}
