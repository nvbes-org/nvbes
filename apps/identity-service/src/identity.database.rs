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

    seed_public_client(
        pool,
        PublicClientSeed {
            tenant_id: system_tenant_id,
            client_id: "cloud-web",
            name: "Cloud Web",
            redirect_uris: vec![
                "http://localhost:5173/callback".to_string(),
                "http://localhost:3001/callback".to_string(),
                "https://app.staging.nvbes.example/callback".to_string(),
            ],
        },
    )
    .await?;
    seed_public_client(
        pool,
        PublicClientSeed {
            tenant_id: system_tenant_id,
            client_id: "console-web",
            name: "Console Web",
            redirect_uris: vec![
                "http://localhost:5175/callback".to_string(),
                "https://developers.staging.nvbes.example/callback".to_string(),
            ],
        },
    )
    .await?;
    seed_public_client(
        pool,
        PublicClientSeed {
            tenant_id: system_tenant_id,
            client_id: "account-web",
            name: "Account Web",
            redirect_uris: account_web_redirect_uris()?,
        },
    )
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

struct PublicClientSeed<'a> {
    tenant_id: uuid::Uuid,
    client_id: &'a str,
    name: &'a str,
    redirect_uris: Vec<String>,
}

async fn seed_public_client(pool: &PgPool, seed: PublicClientSeed<'_>) -> anyhow::Result<()> {
    let client_secret_hash =
        nvbes_product_identity::oauth::hash_client_secret("").map_err(anyhow::Error::from)?;
    sqlx::query(
        r#"
        INSERT INTO oauth_clients (
            client_id, client_secret_hash, name, redirect_uris, tenant_id,
            owner_scope_type, owner_scope_id, client_type
        )
        VALUES ($1, $2, $3, $4, $5, 'tenant', $5, 'public')
        ON CONFLICT (client_id) DO UPDATE
        SET client_secret_hash = EXCLUDED.client_secret_hash,
            name = EXCLUDED.name,
            redirect_uris = EXCLUDED.redirect_uris,
            client_type = 'public'
        "#,
    )
    .bind(seed.client_id)
    .bind(client_secret_hash)
    .bind(seed.name)
    .bind(seed.redirect_uris)
    .bind(seed.tenant_id)
    .execute(pool)
    .await?;

    Ok(())
}

fn account_web_redirect_uris() -> anyhow::Result<Vec<String>> {
    const ENV_NAME: &str = "NVBES_ACCOUNT_WEB_OAUTH_REDIRECT_URIS";

    match std::env::var(ENV_NAME) {
        Ok(value) => parse_account_web_redirect_uris(&value, ENV_NAME),
        Err(std::env::VarError::NotPresent)
            if matches!(
                std::env::var("NVBES_ENV").as_deref(),
                Ok("production" | "staging")
            ) =>
        {
            anyhow::bail!("{ENV_NAME} is required in staging and production")
        }
        Err(std::env::VarError::NotPresent) => {
            Ok(vec!["http://localhost:3001/oauth/callback".to_string()])
        }
        Err(error) => Err(anyhow::anyhow!("{ENV_NAME} could not be read: {error}")),
    }
}

fn parse_account_web_redirect_uris(value: &str, source: &str) -> anyhow::Result<Vec<String>> {
    let redirect_uris = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if redirect_uris.is_empty() {
        anyhow::bail!("{source} must contain at least one redirect URI");
    }
    for redirect_uri in &redirect_uris {
        let parsed = url::Url::parse(redirect_uri)
            .map_err(|error| anyhow::anyhow!("{source} contains an invalid URI: {error}"))?;
        if !matches!(parsed.scheme(), "http" | "https") || parsed.fragment().is_some() {
            anyhow::bail!("{source} redirect URIs must use HTTP(S) and cannot contain a fragment");
        }
    }
    Ok(redirect_uris)
}

async fn seed_confidential_client(
    pool: &PgPool,
    seed: ConfidentialClientSeed<'_>,
) -> anyhow::Result<()> {
    let client_id =
        std::env::var(seed.client_id_env).unwrap_or_else(|_| seed.default_client_id.to_string());
    let client_secret = std::env::var(seed.client_secret_env)
        .unwrap_or_else(|_| seed.default_client_secret.to_string());
    let client_secret_hash = nvbes_product_identity::oauth::hash_client_secret(&client_secret)
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

#[cfg(test)]
mod tests {
    use super::parse_account_web_redirect_uris;

    #[test]
    fn account_web_redirect_uris_are_explicit_and_validated() {
        let values = parse_account_web_redirect_uris(
            "http://localhost:3001/oauth/callback, https://account.example/oauth/callback",
            "test",
        )
        .expect("valid redirect URIs");

        assert_eq!(values.len(), 2);
        assert_eq!(values[0], "http://localhost:3001/oauth/callback");
    }

    #[test]
    fn account_web_redirect_uris_reject_fragments() {
        let error =
            parse_account_web_redirect_uris("https://account.example/oauth/callback#token", "test")
                .expect_err("fragments must be rejected");

        assert!(error.to_string().contains("cannot contain a fragment"));
    }
}
