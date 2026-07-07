use crate::{
    grpc::pb::nvbes::developer::v1 as developer,
    test_support::{
        cleanup_tenant, has_developer_contract_schema, seed_developer_fixture, test_pool,
    },
};

#[tokio::test]
async fn sandbox_creation_is_idempotent_and_updates_profile() {
    let pool = test_pool();
    if !has_developer_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer contract schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "sandbox").await;
    let first = super::create_sandbox(
        &pool,
        developer::CreateSandboxRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            template: String::new(),
        },
    )
    .await
    .expect("sandbox should be created");

    assert_eq!(first.tenant_id, fixture.tenant_id.to_string());
    assert_eq!(first.status, "active");
    assert_eq!(first.data_profile, "minimal");
    assert!(first.sandbox_slug.starts_with("sandbox-"));

    let second = super::create_sandbox(
        &pool,
        developer::CreateSandboxRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            template: "oauth".to_string(),
        },
    )
    .await
    .expect("existing sandbox profile should be updated");

    assert_eq!(second.sandbox_id, first.sandbox_id);
    assert_eq!(second.data_profile, "oauth");

    cleanup_tenant(&pool, fixture.tenant_id).await;
}
