use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn cleanup(pool: &PgPool, tenant_id: Uuid) {
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(pool)
        .await
        .ok();
}

pub(super) async fn seed_service_client(pool: &PgPool) -> (Uuid, String, String, Uuid, Uuid, Uuid) {
    crate::test_support::ensure_test_redis().await;
    crate::test_support::ensure_test_database(pool).await;

    let tenant_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let client_uuid = Uuid::new_v4();
    let client_id = format!("gxoc_test_{}", Uuid::new_v4().simple());
    let client_secret = format!("gxo_test_{}", Uuid::new_v4().simple());
    let client_secret_hash =
        crate::domains::oauth::hash_client_secret(&client_secret).expect("hash should work");
    let now = Utc::now();

    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
        VALUES ($1, 'team', 'OAuth Test Tenant', $2, 'active', 'standard', $3, $3)
        "#,
    )
    .bind(tenant_id)
    .bind(format!("tenant-{}", tenant_id))
    .bind(now)
    .execute(pool)
    .await
    .expect("tenant insert should succeed");

    sqlx::query(
        r#"
        INSERT INTO workspaces (id, tenant_id, name, workspace_type, plan_code, created_at, updated_at)
        VALUES ($1, $2, 'OAuth Test Workspace', 'team', 'team_plus', $3, $3)
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
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
        VALUES ($1, $2, 'service_account', 'active', 'Drive CI Robot', $3, $3)
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
          secret_hash,
          public_key_jwk,
          last_rotated_at,
          created_at,
          updated_at
        )
        VALUES ($1, $2, $3, NULL, 'Drive CI Robot', 'test service account', 'oauth_client_credentials', $4, NULL, NULL, $5, $5, $5)
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
        INSERT INTO workspace_memberships (workspace_id, principal_id, role, status, source, created_at, updated_at)
        VALUES ($1, $2, 'member', 'active', 'system', $3, $3)
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .bind(now)
    .execute(pool)
    .await
    .expect("workspace membership insert should succeed");

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
        VALUES ($1, $2, $3, 'Drive API client', ARRAY['https://example.com/callback'], $4, 'workspace', $5, 'service', NULL, $6, $6)
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
        VALUES (
          $1,
          'workspace',
          $2,
          ARRAY['drive.files.read', 'drive.workspace.read'],
          ARRAY['nvbes-drive-api'],
          ARRAY['drive'],
          'aal1',
          'active',
          $3
        )
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
        client_id,
        client_secret,
        principal_id,
        workspace_id,
        client_uuid,
    )
}
