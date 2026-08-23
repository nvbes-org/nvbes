use uuid::Uuid;

use crate::{
    grpc::pb::nvbes::developer::v1 as developer,
    test_support::{has_developer_contract_schema, test_pool},
};

#[tokio::test]
async fn scope_selection_round_trip_persists_allowed_audiences() {
    let pool = test_pool();
    if !has_developer_contract_schema(&pool).await {
        eprintln!("skipping test: local database is missing Developer contract schema");
        return;
    }

    let scope_key = format!("contract.scope.{}", Uuid::new_v4().simple());
    let created = super::create_scope(
        &pool,
        developer::CreateScopeRequest {
            context: None,
            scope_key: scope_key.clone(),
            display_name: "Contract scope".to_string(),
            description: "Scope selected by contract tests".to_string(),
            risk: "HIGH".to_string(),
            owner_team: "identity".to_string(),
            lifecycle: String::new(),
            allowed_audiences: vec![
                "https://api.nvbes.test".to_string(),
                "https://console.nvbes.test".to_string(),
            ],
        },
    )
    .await
    .expect("scope should be created");

    assert_eq!(created.scope_key, scope_key);
    assert_eq!(created.risk, "high");
    assert_eq!(created.lifecycle, "proposed");
    assert_eq!(created.allowed_audiences.len(), 2);

    let listed = super::list_scopes(&pool)
        .await
        .expect("scopes should be listed");
    let listed_scope = listed
        .scopes
        .iter()
        .find(|scope| scope.scope_key == scope_key)
        .expect("created scope should be returned");
    assert_eq!(listed_scope.allowed_audiences, created.allowed_audiences);

    let deleted = super::delete_scope(&pool, scope_key.clone())
        .await
        .expect("scope should be deleted");
    assert_eq!(deleted.scope_key, scope_key);
}
