use uuid::Uuid;

use crate::{
    grpc::pb::nvbes::developer::v1 as developer,
    test_support::{
        cleanup_tenant, has_developer_contract_schema, seed_developer_fixture, test_pool,
    },
};

#[tokio::test]
async fn token_inspection_result_is_recorded_for_actor_and_tenant() {
    let pool = test_pool();
    if !has_developer_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer contract schema");
        return;
    }

    let fixture = seed_developer_fixture(&pool, "token-debug").await;
    let session = super::record_token_debug_session(
        &pool,
        fixture.principal_id,
        developer::RecordTokenDebugSessionRequest {
            context: None,
            tenant_id: fixture.tenant_id.to_string(),
            token_hash_prefix: "fixture-prefix".to_string(),
            active: true,
            access_decision: "allowed".to_string(),
        },
    )
    .await
    .expect("token debug session should be recorded");

    assert_eq!(session.tenant_id, fixture.tenant_id.to_string());
    assert_eq!(session.actor_principal_id, fixture.principal_id.to_string());
    assert_eq!(session.token_hash_prefix, "fixture-prefix");
    assert!(session.active);
    assert_eq!(session.access_decision, "allowed");
    assert!(Uuid::parse_str(&session.session_id).is_ok());

    cleanup_tenant(&pool, fixture.tenant_id).await;
}
