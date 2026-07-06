use sqlx::postgres::PgPool;

pub type Database = PgPool;

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

pub async fn ensure_default_oauth_clients_seeded(pool: &PgPool) -> anyhow::Result<()> {
    use uuid::Uuid;

    // 1. Ensure system tenant exists
    let system_tenant_id = Uuid::nil(); // 00000000-0000-0000-0000-000000000000
    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier)
        VALUES ($1, 'system', 'System Tenant', 'system-tenant', 'active', 'standard')
        ON CONFLICT DO NOTHING
        "#,
    )
    .bind(system_tenant_id)
    .execute(pool)
    .await?;

    // 2. Seed cloud-web (public client)
    let drive_web_secret_hash =
        nvbes_product_account::oauth::hash_client_secret("").map_err(anyhow::Error::from)?;
    let drive_web_redirect_uris = vec![
        "http://localhost:5173/callback".to_string(),
        "http://localhost:3001/callback".to_string(),
        "https://app.staging.nvbes.example/callback".to_string(),
    ];
    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
            client_id, client_secret_hash, name, redirect_uris, tenant_id,
            owner_scope_type, owner_scope_id, client_type
        )
        VALUES ('cloud-web', $1, 'Drive Web', $2, $3, 'tenant', $3, 'public')
        ON CONFLICT (client_id) DO UPDATE
        SET client_secret_hash = EXCLUDED.client_secret_hash,
            redirect_uris = EXCLUDED.redirect_uris
        "#,
    )
    .bind(drive_web_secret_hash)
    .bind(&drive_web_redirect_uris)
    .bind(system_tenant_id)
    .execute(pool)
    .await?;

    // 3. Seed console-web (public client)
    let developer_web_secret_hash =
        nvbes_product_account::oauth::hash_client_secret("").map_err(anyhow::Error::from)?;
    let developer_web_redirect_uris = vec![
        "http://localhost:5175/callback".to_string(),
        "https://developers.staging.nvbes.example/callback".to_string(),
    ];
    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
            client_id, client_secret_hash, name, redirect_uris, tenant_id,
            owner_scope_type, owner_scope_id, client_type
        )
        VALUES ('console-web', $1, 'Console Web', $2, $3, 'tenant', $3, 'public')
        ON CONFLICT (client_id) DO UPDATE
        SET client_secret_hash = EXCLUDED.client_secret_hash,
            redirect_uris = EXCLUDED.redirect_uris
        "#,
    )
    .bind(developer_web_secret_hash)
    .bind(&developer_web_redirect_uris)
    .bind(system_tenant_id)
    .execute(pool)
    .await?;

    // 4. Seed cloud-worker (confidential client)
    let worker_secret = std::env::var("NVBES_ACCOUNT_SERVICE_CLIENT_SECRET")
        .unwrap_or_else(|_| "cloud-worker-secret-key-12345".to_string());
    let drive_worker_secret_hash = nvbes_product_account::oauth::hash_client_secret(&worker_secret)
        .map_err(anyhow::Error::from)?;
    let drive_worker_redirect_uris: Vec<String> = vec![];
    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
            client_id, client_secret_hash, name, redirect_uris, tenant_id,
            owner_scope_type, owner_scope_id, client_type
        )
        VALUES ('cloud-worker', $1, 'Drive Worker', $2, $3, 'tenant', $3, 'confidential')
        ON CONFLICT (client_id) DO UPDATE
        SET client_secret_hash = EXCLUDED.client_secret_hash
        "#,
    )
    .bind(drive_worker_secret_hash)
    .bind(&drive_worker_redirect_uris)
    .bind(system_tenant_id)
    .execute(pool)
    .await?;

    Ok(())
}
