use axum::{Extension, Json, extract::Path, extract::State};

use super::support::{developer_auth, seed_developer_admin, test_state};

#[tokio::test]
async fn scope_registry_crud() {
    let pool = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&pool).await;
    let fixture =
        seed_developer_admin(&pool, "Scope Test Tenant", "scope-test", "Scope User").await;
    let auth = developer_auth(
        fixture.tenant_id,
        fixture.principal_id,
        "scope-admin@example.com",
        "Scope Admin User",
    );
    let state = test_state(&pool).await;

    let create_response = crate::domains::developer::routes::oauth::create_scope(
        State(state.clone()),
        Extension(auth.clone()),
        Json(crate::domains::developer::types::CreateScopeInput {
            scope_key: "test.scope.write".to_string(),
            display_name: "Test Scope Write".to_string(),
            description: "Allows writing test data".to_string(),
            risk: "medium".to_string(),
            owner_team: "Platform Security".to_string(),
            lifecycle: Some("active".to_string()),
            allowed_audiences: vec!["https://api.test.com".to_string()],
        }),
    )
    .await
    .expect("scope should be created");

    assert_eq!(create_response.0.scope_key, "test.scope.write");
    assert_eq!(create_response.0.display_name, "Test Scope Write");
    assert_eq!(create_response.0.risk, "medium");
    assert_eq!(create_response.0.lifecycle, "active");
    assert!(scope_registry_exists(&pool).await);
    assert!(scope_metadata_exists(&pool).await);

    let list_response = crate::domains::developer::routes::oauth::list_scopes(
        State(state.clone()),
        Extension(auth.clone()),
    )
    .await
    .expect("scopes should list");
    assert!(
        list_response
            .0
            .scopes
            .iter()
            .any(|scope| scope.scope_key == "test.scope.write")
    );

    let update_response = crate::domains::developer::routes::oauth::update_scope(
        State(state.clone()),
        Extension(auth.clone()),
        Path("test.scope.write".to_string()),
        Json(crate::domains::developer::types::UpdateScopeInput {
            display_name: "Test Scope Write Updated".to_string(),
            description: "Allows writing test data updated".to_string(),
            risk: "high".to_string(),
            owner_team: "Platform Security Team".to_string(),
            lifecycle: "active".to_string(),
            allowed_audiences: vec![
                "https://api.test.com".to_string(),
                "https://api2.test.com".to_string(),
            ],
        }),
    )
    .await
    .expect("scope should update");

    assert_eq!(update_response.0.display_name, "Test Scope Write Updated");
    assert_eq!(update_response.0.risk, "high");
    assert!(scope_requires_admin_consent(&pool).await);

    let _ = crate::domains::developer::routes::oauth::delete_scope(
        State(state.clone()),
        Extension(auth),
        Path("test.scope.write".to_string()),
    )
    .await
    .expect("scope should delete");

    assert!(!scope_registry_exists(&pool).await);
    assert!(!scope_metadata_exists(&pool).await);
}

async fn scope_registry_exists(pool: &sqlx::PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM developer_scope_registry WHERE scope_key = 'test.scope.write')",
    )
    .fetch_one(pool)
    .await
    .expect("scope registry existence should be readable")
}

async fn scope_metadata_exists(pool: &sqlx::PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM oauth_scope_metadata WHERE scope = 'test.scope.write')",
    )
    .fetch_one(pool)
    .await
    .expect("scope metadata existence should be readable")
}

async fn scope_requires_admin_consent(pool: &sqlx::PgPool) -> bool {
    sqlx::query_scalar::<_, bool>(
        "SELECT requires_admin_consent FROM oauth_scope_metadata WHERE scope = 'test.scope.write'",
    )
    .fetch_one(pool)
    .await
    .expect("scope metadata should be readable")
}
