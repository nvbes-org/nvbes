use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{domains::oauth, http::error::AppError};

const FIXTURE_CLIENT_SECRET: &str = "sandbox-secret";

pub(super) async fn provision_sandbox(
    db: &PgPool,
    sandbox_tenant_id: Uuid,
    data_profile: &str,
) -> Result<(), AppError> {
    let mut tx = db.begin().await?;

    purge_sandbox_data(&mut tx, sandbox_tenant_id).await?;
    let actor_id = seed_sandbox_actor(&mut tx, sandbox_tenant_id).await?;

    if matches!(data_profile, "oauth" | "full") {
        seed_oauth_profile(&mut tx, sandbox_tenant_id, actor_id).await?;
    }

    if data_profile == "full" {
        seed_full_profile(&mut tx, sandbox_tenant_id, actor_id).await?;
    }

    tx.commit().await.map_err(AppError::from)
}

async fn purge_sandbox_data(
    tx: &mut Transaction<'_, Postgres>,
    sandbox_tenant_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM developer_health_checks WHERE tenant_id = $1")
        .bind(sandbox_tenant_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM developer_webhook_endpoints WHERE tenant_id = $1")
        .bind(sandbox_tenant_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM oauth_clients WHERE tenant_id = $1")
        .bind(sandbox_tenant_id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM principals WHERE tenant_id = $1")
        .bind(sandbox_tenant_id)
        .execute(&mut **tx)
        .await?;

    Ok(())
}

async fn seed_sandbox_actor(
    tx: &mut Transaction<'_, Postgres>,
    sandbox_tenant_id: Uuid,
) -> Result<Uuid, AppError> {
    let actor_id = Uuid::new_v4();
    let email = format!("sandbox.{}@example.test", sandbox_tenant_id.simple());

    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
        VALUES ($1, $2, 'human', 'active', 'Sandbox Developer')
        "#,
    )
    .bind(actor_id)
    .bind(sandbox_tenant_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO users (
          principal_id,
          email,
          firstname,
          lastname,
          username,
          email_verified_at,
          status
        )
        VALUES ($1, $2, 'Sandbox', 'Developer', $3, now(), 'active')
        "#,
    )
    .bind(actor_id)
    .bind(&email)
    .bind(format!("sandbox-{}", sandbox_tenant_id.simple()))
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO tenant_memberships (
          tenant_id,
          principal_id,
          principal_kind,
          role,
          status,
          source
        )
        VALUES ($1, $2, 'human', 'admin', 'active', 'system')
        "#,
    )
    .bind(sandbox_tenant_id)
    .bind(actor_id)
    .execute(&mut **tx)
    .await?;

    Ok(actor_id)
}

async fn seed_oauth_profile(
    tx: &mut Transaction<'_, Postgres>,
    sandbox_tenant_id: Uuid,
    actor_id: Uuid,
) -> Result<(), AppError> {
    let oauth_client_id = Uuid::new_v4();
    let client_id = format!("sandbox-{}", sandbox_tenant_id.simple());
    let secret_hash = oauth::hash_client_secret(FIXTURE_CLIENT_SECRET)?;

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
          requires_admin_consent
        )
        VALUES (
          $1,
          $2,
          $3,
          'Sandbox OAuth Client',
          ARRAY['http://localhost:5175/oauth/callback'],
          $4,
          'tenant',
          $4,
          'confidential',
          false
        )
        "#,
    )
    .bind(oauth_client_id)
    .bind(&client_id)
    .bind(secret_hash)
    .bind(sandbox_tenant_id)
    .execute(&mut **tx)
    .await?;

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
          status
        )
        VALUES ($1, 'tenant', $2, ARRAY['openid', 'profile', 'email'], '{}', '{}', 'aal1', 'active')
        "#,
    )
    .bind(oauth_client_id)
    .bind(sandbox_tenant_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO developer_consent_screens (
          tenant_id,
          client_id,
          product_name,
          description,
          updated_by
        )
        VALUES ($1, $2, 'Sandbox App', 'Integration-test consent screen.', $3)
        "#,
    )
    .bind(sandbox_tenant_id)
    .bind(&client_id)
    .bind(actor_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn seed_full_profile(
    tx: &mut Transaction<'_, Postgres>,
    sandbox_tenant_id: Uuid,
    actor_id: Uuid,
) -> Result<(), AppError> {
    let endpoint_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO developer_webhook_endpoints (
          id,
          tenant_id,
          name,
          url,
          signing_secret_ciphertext,
          signing_secret_last4,
          created_by
        )
        VALUES (
          $1,
          $2,
          'Sandbox Webhook',
          'https://example.test/nvbes/webhook',
          'sandbox-signing-secret',
          'cret',
          $3
        )
        "#,
    )
    .bind(endpoint_id)
    .bind(sandbox_tenant_id)
    .bind(actor_id)
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO developer_webhook_deliveries (
          endpoint_id,
          tenant_id,
          event_id,
          event_type,
          status,
          attempt_count,
          response_status,
          delivered_at
        )
        VALUES ($1, $2, $3, 'user.created', 'delivered', 1, 200, now())
        "#,
    )
    .bind(endpoint_id)
    .bind(sandbox_tenant_id)
    .bind(Uuid::new_v4())
    .execute(&mut **tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO developer_health_checks (
          tenant_id,
          target_type,
          target_id,
          check_kind,
          status,
          summary
        )
        VALUES ($1, 'sandbox', $2, 'fixtures', 'passing', 'Sandbox fixtures are ready.')
        "#,
    )
    .bind(sandbox_tenant_id)
    .bind(sandbox_tenant_id.to_string())
    .execute(&mut **tx)
    .await?;

    Ok(())
}
