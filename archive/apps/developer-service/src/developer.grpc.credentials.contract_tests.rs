use chrono::Duration;
use uuid::Uuid;

use crate::{
    grpc::pb::nvbes::developer::v1 as developer,
    test_support::{
        cleanup_tenant, has_developer_contract_schema, request_context, seed_developer_fixture,
        test_pool,
    },
};

#[tokio::test]
async fn credential_summaries_and_stale_secret_count_are_tenant_scoped() {
    let pool = test_pool();
    if !has_developer_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer contract schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "credentials").await;
    let oauth_client_id = Uuid::new_v4();
    let client_id = format!("contract-{}", fixture.tenant_id.simple());

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
        VALUES ($1, $2, 'hash', 'Contract Client', '{}', $3, 'tenant', $3, 'confidential', false)
        "#,
    )
    .bind(oauth_client_id)
    .bind(&client_id)
    .bind(fixture.tenant_id)
    .execute(&pool)
    .await
    .expect("oauth client should be seeded");

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
        VALUES ($1, 'tenant', $2, ARRAY['openid', 'profile'], '{}', '{}', 'aal1', 'active')
        "#,
    )
    .bind(oauth_client_id)
    .bind(fixture.tenant_id)
    .execute(&pool)
    .await
    .expect("oauth policy should be seeded");

    let active_created_at = fixture.now - Duration::days(91);
    let overlap_expires_at = fixture.now - Duration::days(1);
    sqlx::query(
        r#"
        INSERT INTO developer_client_secret_versions (
          tenant_id,
          client_id,
          status,
          client_secret_hash,
          secret_last4,
          created_at
        )
        VALUES ($1, $2, 'active', 'active-hash', '1234', $3)
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(&client_id)
    .bind(active_created_at)
    .execute(&pool)
    .await
    .expect("active secret version should be seeded");

    sqlx::query(
        r#"
        INSERT INTO developer_client_secret_versions (
          tenant_id,
          client_id,
          status,
          client_secret_hash,
          secret_last4,
          expires_at
        )
        VALUES ($1, $2, 'overlap', 'overlap-hash', '5678', $3)
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(&client_id)
    .bind(overlap_expires_at)
    .execute(&pool)
    .await
    .expect("overlap secret version should be seeded");

    let summaries = super::list_credential_summaries(
        &pool,
        developer::ListCredentialSummariesRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
        },
    )
    .await
    .expect("credential summaries should be listed");
    assert_eq!(summaries.credentials.len(), 2);
    assert!(
        summaries
            .credentials
            .iter()
            .all(|credential| credential.client_id == client_id)
    );
    assert!(
        summaries
            .credentials
            .iter()
            .all(|credential| credential.scopes == vec!["openid", "profile"])
    );

    let stale = super::count_stale_secrets(
        &pool,
        developer::CountStaleSecretsRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
        },
    )
    .await
    .expect("stale secrets should be counted");
    assert_eq!(stale.stale_secret_count, 2);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

#[tokio::test]
async fn client_secret_version_validation_accepts_only_live_versions() {
    let pool = test_pool();
    if !has_developer_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer contract schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "secret-validation").await;
    let oauth_client_id = Uuid::new_v4();
    let client_id = format!("secret-validation-{}", fixture.tenant_id.simple());

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
        VALUES ($1, $2, 'hash', 'Secret Validation Client', '{}', $3, 'tenant', $3, 'confidential', false)
        "#,
    )
    .bind(oauth_client_id)
    .bind(&client_id)
    .bind(fixture.tenant_id)
    .execute(&pool)
    .await
    .expect("oauth client should be seeded");

    let live_secret = "gxo_live_secret_1234567890";
    let revoked_secret = "gxo_revoked_secret_1234567890";
    let live_hash =
        nvbes_product_identity::oauth::hash_client_secret(live_secret).expect("hash live secret");
    let revoked_hash = nvbes_product_identity::oauth::hash_client_secret(revoked_secret)
        .expect("hash revoked secret");

    sqlx::query(
        r#"
        INSERT INTO developer_client_secret_versions (
          tenant_id,
          client_id,
          status,
          client_secret_hash,
          secret_last4,
          expires_at
        )
        VALUES
          ($1, $2, 'overlap', $3, '7890', NOW() + INTERVAL '1 hour'),
          ($1, $2, 'revoked', $4, '7890', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(&client_id)
    .bind(live_hash)
    .bind(revoked_hash)
    .execute(&pool)
    .await
    .expect("secret versions should be seeded");

    let live = super::verify_client_secret_version(
        &pool,
        developer::VerifyClientSecretVersionRequest {
            context: Some(request_context(fixture.tenant_id, fixture.principal_id)),
            tenant_id: fixture.tenant_id.to_string(),
            client_id: client_id.clone(),
            client_secret: live_secret.to_string(),
        },
    )
    .await
    .expect("live secret should be checked");
    assert!(live.valid);

    let revoked = super::verify_client_secret_version(
        &pool,
        developer::VerifyClientSecretVersionRequest {
            context: Some(request_context(fixture.tenant_id, fixture.principal_id)),
            tenant_id: fixture.tenant_id.to_string(),
            client_id,
            client_secret: revoked_secret.to_string(),
        },
    )
    .await
    .expect("revoked secret should be checked");
    assert!(!revoked.valid);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}
