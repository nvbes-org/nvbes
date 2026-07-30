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

    seed_confidential_client(
        pool,
        ConfidentialClientSeed {
            tenant_id: system_tenant_id,
            client_id_env: "NVBES_CLOUD_IDENTITY_CLIENT_ID",
            client_secret_env: "NVBES_CLOUD_IDENTITY_CLIENT_SECRET",
            default_client_id: "cloud-worker",
            default_client_secret: "cloud-worker-secret-key-12345",
            name: "Cloud Worker",
        },
    )
    .await?;
    seed_confidential_client(
        pool,
        ConfidentialClientSeed {
            tenant_id: system_tenant_id,
            client_id_env: "NVBES_DEVELOPER_IDENTITY_CLIENT_ID",
            client_secret_env: "NVBES_DEVELOPER_IDENTITY_CLIENT_SECRET",
            default_client_id: "developer-service",
            default_client_secret: "developer-service-secret-key-12345",
            name: "Developer Service",
        },
    )
    .await?;
    seed_confidential_client(
        pool,
        ConfidentialClientSeed {
            tenant_id: system_tenant_id,
            client_id_env: "NVBES_BILLING_IDENTITY_CLIENT_ID",
            client_secret_env: "NVBES_BILLING_IDENTITY_CLIENT_SECRET",
            default_client_id: "billing-service",
            default_client_secret: "billing-service-secret-key-12345",
            name: "Billing Service",
        },
    )
    .await?;
    seed_confidential_client(
        pool,
        ConfidentialClientSeed {
            tenant_id: system_tenant_id,
            client_id_env: "NVBES_GATEWAY_IDENTITY_CLIENT_ID",
            client_secret_env: "NVBES_GATEWAY_IDENTITY_CLIENT_SECRET",
            default_client_id: "gateway-cloud",
            default_client_secret: "gateway-cloud-secret-key-12345",
            name: "Gateway Cloud",
        },
    )
    .await?;
    seed_confidential_client(
        pool,
        ConfidentialClientSeed {
            tenant_id: system_tenant_id,
            client_id_env: "NVBES_BACKOFFICE_IDENTITY_CLIENT_ID",
            client_secret_env: "NVBES_BACKOFFICE_IDENTITY_CLIENT_SECRET",
            default_client_id: "backoffice-service",
            default_client_secret: "development-backoffice-introspection-secret",
            name: "Backoffice Service",
        },
    )
    .await?;

    Ok(())
}

struct ConfidentialClientSeed<'a> {
    tenant_id: uuid::Uuid,
    client_id_env: &'a str,
    client_secret_env: &'a str,
    default_client_id: &'a str,
    default_client_secret: &'a str,
    name: &'a str,
}

async fn seed_confidential_client(
    pool: &PgPool,
    seed: ConfidentialClientSeed<'_>,
) -> anyhow::Result<()> {
    let client_id =
        std::env::var(seed.client_id_env).unwrap_or_else(|_| seed.default_client_id.to_string());
    let client_secret = std::env::var(seed.client_secret_env)
        .unwrap_or_else(|_| seed.default_client_secret.to_string());
    let client_secret_hash = nvbes_product_account::oauth::hash_client_secret(&client_secret)
        .map_err(anyhow::Error::from)?;

    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
            client_id, client_secret_hash, name, redirect_uris, tenant_id,
            owner_scope_type, owner_scope_id, client_type
        )
        VALUES ($1, $2, $3, ARRAY[]::text[], $4, 'tenant', $4, 'confidential')
        ON CONFLICT (client_id) DO UPDATE
        SET client_secret_hash = EXCLUDED.client_secret_hash,
            name = EXCLUDED.name
        "#,
    )
    .bind(client_id)
    .bind(client_secret_hash)
    .bind(seed.name)
    .bind(seed.tenant_id)
    .execute(pool)
    .await?;

    Ok(())
}
