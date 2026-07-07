use uuid::Uuid;

use crate::{
    grpc::pb::nvbes::developer::v1 as developer,
    test_support::{
        cleanup_tenant, has_developer_contract_schema, request_context, seed_developer_fixture,
        test_pool,
    },
};

#[tokio::test]
async fn client_metadata_returns_developer_owned_enrichment_by_client_id() {
    let pool = test_pool();
    if !has_developer_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer contract schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "client-metadata").await;
    let oauth_client_id = Uuid::new_v4();
    let client_id = format!("metadata-{}", fixture.tenant_id.simple());

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
        VALUES ($1, $2, 'hash', 'Metadata Client', '{}', $3, 'tenant', $3, 'confidential', false)
        "#,
    )
    .bind(oauth_client_id)
    .bind(&client_id)
    .bind(fixture.tenant_id)
    .execute(&pool)
    .await
    .expect("oauth client should be seeded");

    sqlx::query(
        "INSERT INTO developer_marketplace_apps (tenant_id, client_id, status, submitted_by) VALUES ($1, $2, 'approved', $3)",
    )
    .bind(fixture.tenant_id)
    .bind(&client_id)
    .bind(fixture.principal_id)
    .execute(&pool)
    .await
    .expect("marketplace app should be seeded");

    sqlx::query(
        "INSERT INTO developer_consent_screens (tenant_id, client_id, product_name, description, updated_by) VALUES ($1, $2, 'Metadata App', 'Metadata app consent', $3)",
    )
    .bind(fixture.tenant_id)
    .bind(&client_id)
    .bind(fixture.principal_id)
    .execute(&pool)
    .await
    .expect("consent screen should be seeded");

    sqlx::query(
        "INSERT INTO developer_health_checks (tenant_id, target_type, target_id, check_kind, status, summary) VALUES ($1, 'oauth_client', $2, 'configuration', 'warning', 'warning')",
    )
    .bind(fixture.tenant_id)
    .bind(&client_id)
    .execute(&pool)
    .await
    .expect("health check should be seeded");

    let response = super::client_metadata(
        &pool,
        developer::GetClientMetadataRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            client_ids: vec![client_id.clone(), "missing-client".to_string()],
        },
    )
    .await
    .expect("client metadata should be listed");

    assert_eq!(response.metadata.len(), 2);
    let metadata = response
        .metadata
        .iter()
        .find(|metadata| metadata.client_id == client_id)
        .expect("seeded client metadata should be returned");
    assert_eq!(metadata.marketplace_status, "approved");
    assert!(metadata.consent_screen_configured);
    assert_eq!(metadata.health_status, "warning");

    let missing = response
        .metadata
        .iter()
        .find(|metadata| metadata.client_id == "missing-client")
        .expect("missing client default metadata should be returned");
    assert_eq!(missing.marketplace_status, "");
    assert!(!missing.consent_screen_configured);
    assert_eq!(missing.health_status, "unknown");

    cleanup_tenant(&pool, fixture.tenant_id).await;
}

#[tokio::test]
async fn public_consent_screen_returns_display_without_actor_context() {
    let pool = test_pool();
    if !has_developer_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer contract schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "public-consent").await;
    let client_id = format!("public-consent-{}", fixture.tenant_id.simple());

    sqlx::query(
        r#"
        INSERT INTO developer_consent_screens (
          tenant_id,
          client_id,
          product_name,
          logo_url,
          support_url,
          privacy_url,
          terms_url,
          description,
          brand_color,
          custom_css,
          help_text,
          updated_by
        )
        VALUES (
          $1,
          $2,
          'Public Consent App',
          'https://cdn.example/logo.png',
          'https://example.test/support',
          'https://example.test/privacy',
          'https://example.test/terms',
          'Consent screen description',
          '#123456',
          '',
          'Need help?',
          $3
        )
        "#,
    )
    .bind(fixture.tenant_id)
    .bind(&client_id)
    .bind(fixture.principal_id)
    .execute(&pool)
    .await
    .expect("consent screen should be seeded");

    let screen = super::super::consent::get_public_consent_screen(
        &pool,
        developer::GetPublicConsentScreenRequest {
            context: Some(request_context(fixture.tenant_id, fixture.principal_id)),
            tenant_id: fixture.tenant_id.to_string(),
            client_id: client_id.clone(),
        },
    )
    .await
    .expect("public consent screen should be readable");

    assert_eq!(screen.client_id, client_id);
    assert_eq!(screen.product_name, "Public Consent App");
    assert_eq!(screen.logo_url, "https://cdn.example/logo.png");
    assert_eq!(screen.description, "Consent screen description");
    assert!(screen.configured);

    cleanup_tenant(&pool, fixture.tenant_id).await;
}
