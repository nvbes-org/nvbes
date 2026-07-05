use crate::domains::service_accounts::routes::{revoke_oauth_client, suspend_service_account};
use axum::extract::{Path, State};
use sqlx::PgPool;
use uuid::Uuid;

#[path = "identity.domains.service_accounts.routes.tests.cleanup.rs"]
mod cleanup_support;
#[path = "identity.domains.service_accounts.routes.tests.seed.rs"]
mod seed;
#[path = "identity.domains.service_accounts.routes.tests.session.rs"]
mod session;
#[path = "identity.domains.service_accounts.routes.tests.state.rs"]
mod state_support;

use cleanup_support::cleanup;
use seed::seed_admin_workspace;
use state_support::{
    assert_machine_token_fails, bearer_headers, test_database_url, test_lock, test_state,
};

#[tokio::test]
async fn suspend_service_account_endpoint_blocks_client_credentials_immediately() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    let state = test_state(&pool).await;
    let redis = state.redis.clone();
    let fixture = seed_admin_workspace(&pool, &redis, &state.jwt).await;

    let response = suspend_service_account(
        State(state.clone()),
        bearer_headers(&fixture.admin_token),
        Path((fixture.workspace_id, fixture.service_principal_id)),
    )
    .await
    .expect("service account suspension should succeed");

    assert_eq!(response.0.status, "suspended");
    assert_eq!(response.0.principal_id, fixture.service_principal_id);

    assert_machine_token_fails(&state, &fixture, "service_account_inactive").await;

    cleanup(&pool, &redis, &fixture).await;
}

#[tokio::test]
async fn revoke_oauth_client_endpoint_detaches_client_and_blocks_client_credentials() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    let state = test_state(&pool).await;
    let redis = state.redis.clone();
    let fixture = seed_admin_workspace(&pool, &redis, &state.jwt).await;

    let response = revoke_oauth_client(
        State(state.clone()),
        bearer_headers(&fixture.admin_token),
        Path((
            fixture.workspace_id,
            fixture.service_principal_id,
            fixture.client_id.clone(),
        )),
    )
    .await
    .expect("oauth client revocation should succeed");

    assert_eq!(response.0.principal_id, fixture.service_principal_id);
    assert!(response.0.oauth_clients.is_empty());

    assert_machine_token_fails(&state, &fixture, "invalid_client").await;

    cleanup(&pool, &redis, &fixture).await;
}

#[tokio::test]
async fn service_account_management_rejects_workspace_mismatch() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    let state = test_state(&pool).await;
    let redis = state.redis.clone();
    let fixture = seed_admin_workspace(&pool, &redis, &state.jwt).await;
    let other_workspace_id = Uuid::new_v4();

    let error = suspend_service_account(
        State(state),
        bearer_headers(&fixture.admin_token),
        Path((other_workspace_id, fixture.service_principal_id)),
    )
    .await
    .expect_err("workspace mismatch should be rejected");

    assert_eq!(error.code, "workspace_context_mismatch");

    cleanup(&pool, &redis, &fixture).await;
}
