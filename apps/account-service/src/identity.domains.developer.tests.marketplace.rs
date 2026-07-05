use axum::{Extension, Json, extract::Path, extract::State};
use uuid::Uuid;

use super::support::{developer_auth, seed_developer_admin, test_state};

#[tokio::test]
async fn marketplace_workflow() {
    let pool = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&pool).await;
    let fixture =
        seed_developer_admin(&pool, "Dev Route Test Tenant", "dev-test", "Dev User").await;

    let client_uuid = Uuid::new_v4();
    let client_id = format!("client-{}", Uuid::new_v4().simple());
    let client_secret_hash =
        crate::domains::oauth::hash_client_secret("secret123").expect("hash secret");
    sqlx::query(
        "INSERT INTO oauth_clients (
          id, client_id, client_secret_hash, name, redirect_uris, tenant_id,
          owner_scope_type, owner_scope_id, client_type, created_at, updated_at
        )
        VALUES ($1, $2, $3, 'Test OAuth Client', '{}', $4, 'tenant', $4, 'confidential', $5, $5)",
    )
    .bind(client_uuid)
    .bind(&client_id)
    .bind(client_secret_hash)
    .bind(fixture.tenant_id)
    .bind(fixture.now)
    .execute(&pool)
    .await
    .expect("oauth client should be seeded");

    let auth = developer_auth(
        fixture.tenant_id,
        fixture.principal_id,
        "dev@example.com",
        "Dev User",
    );
    let state = test_state(&pool).await;

    let list_response = crate::domains::developer::routes::oauth::list_marketplace_apps(
        State(state.clone()),
        Extension(auth.clone()),
    )
    .await
    .expect("marketplace apps should list");
    assert_eq!(list_response.0.apps.len(), 0);

    let submit_response = crate::domains::developer::routes::oauth::submit_marketplace_app(
        State(state.clone()),
        Extension(auth.clone()),
        Path(client_id.clone()),
    )
    .await
    .expect("marketplace app should submit");
    assert_eq!(submit_response.0.client_id, client_id);
    assert_eq!(submit_response.0.status, "pending");
    assert_eq!(submit_response.0.review_reason, None);

    let list_response = crate::domains::developer::routes::oauth::list_marketplace_apps(
        State(state.clone()),
        Extension(auth.clone()),
    )
    .await
    .expect("submitted marketplace app should list");
    assert_eq!(list_response.0.apps.len(), 1);
    assert_eq!(list_response.0.apps[0].client_id, client_id);
    assert_eq!(list_response.0.apps[0].status, "pending");

    let review_response = crate::domains::developer::routes::oauth::review_marketplace_app(
        State(state.clone()),
        Extension(auth.clone()),
        Path(client_id.clone()),
        Json(
            crate::domains::developer::types::ReviewMarketplaceAppInput {
                status: "approved".to_string(),
                review_reason: Some("Looks good!".to_string()),
            },
        ),
    )
    .await
    .expect("marketplace app should be approved");
    assert_eq!(review_response.0.client_id, client_id);
    assert_eq!(review_response.0.status, "approved");
    assert_eq!(
        review_response.0.review_reason,
        Some("Looks good!".to_string())
    );

    let review_response = crate::domains::developer::routes::oauth::review_marketplace_app(
        State(state.clone()),
        Extension(auth),
        Path(client_id),
        Json(
            crate::domains::developer::types::ReviewMarketplaceAppInput {
                status: "suspended".to_string(),
                review_reason: Some("Violated policy".to_string()),
            },
        ),
    )
    .await
    .expect("marketplace app should be suspended");
    assert_eq!(review_response.0.status, "suspended");
    assert_eq!(
        review_response.0.review_reason,
        Some("Violated policy".to_string())
    );
}
