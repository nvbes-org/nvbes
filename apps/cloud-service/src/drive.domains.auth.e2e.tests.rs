use axum::{
    body::{self, Body},
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

#[path = "drive.domains.auth.e2e.support.rs"]
mod support;
use support::*;

async fn drive_objects_response(
    app: &axum::Router,
    workspace_id: Uuid,
    access_token: &str,
) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .uri(format!("/workspaces/{workspace_id}/objects"))
                .header("authorization", format!("Bearer {access_token}"))
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("drive app should respond")
}

async fn response_error_code(response: axum::response::Response) -> String {
    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let payload: serde_json::Value =
        serde_json::from_slice(&body).expect("response body should be valid json");
    payload["error"]["code"]
        .as_str()
        .expect("error code should be present")
        .to_string()
}

#[tokio::test]
async fn http_machine_token_from_identity_authorizes_drive_workspace_route() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    if !db_supports_current_schema(&pool).await {
        eprintln!("skipping test: local database is missing current identity/drive schema columns");
        return;
    }

    let identity_state = identity_state(&pool).await;
    let identity_base_url = spawn_identity_server(identity_state).await;
    let (tenant_id, workspace_id, client_id, client_secret, principal_id, owner_email) =
        seed_machine_workspace_context(&pool).await;

    unsafe {
        std::env::set_var("NVBES_IDENTITY_BASE_URL", &identity_base_url);
        std::env::set_var("NVBES_IDENTITY_CLIENT_ID", &client_id);
        std::env::set_var("NVBES_IDENTITY_CLIENT_SECRET", &client_secret);
    }

    let access_token = issue_machine_token(&identity_base_url, &client_id, &client_secret).await;

    let app = drive_app().await;
    let response = drive_objects_response(&app, workspace_id, &access_token).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let payload: serde_json::Value =
        serde_json::from_slice(&body).expect("response body should be valid json");
    assert_eq!(payload["objects"], serde_json::json!([]));

    let shadow_user = sqlx::query_scalar::<_, Option<String>>(
        "SELECT identity_subject FROM users WHERE identity_subject = $1 LIMIT 1",
    )
    .bind(principal_id.to_string())
    .fetch_one(&pool)
    .await
    .expect("shadow user lookup should succeed");
    assert_eq!(
        shadow_user.as_deref(),
        Some(principal_id.to_string().as_str())
    );

    cleanup(&pool, tenant_id, &owner_email).await;
}

#[tokio::test]
async fn http_revoked_machine_client_is_rejected_by_drive_workspace_route() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    if !db_supports_current_schema(&pool).await {
        eprintln!("skipping test: local database is missing current identity/drive schema columns");
        return;
    }

    let identity_state = identity_state(&pool).await;
    let identity_base_url = spawn_identity_server(identity_state).await;
    let (tenant_id, workspace_id, client_id, client_secret, _principal_id, owner_email) =
        seed_machine_workspace_context(&pool).await;

    unsafe {
        std::env::set_var("NVBES_IDENTITY_BASE_URL", &identity_base_url);
        std::env::set_var("NVBES_IDENTITY_CLIENT_ID", &client_id);
        std::env::set_var("NVBES_IDENTITY_CLIENT_SECRET", &client_secret);
    }

    let access_token = issue_machine_token(&identity_base_url, &client_id, &client_secret).await;

    sqlx::query("UPDATE oauth_clients SET revoked_at = NOW() WHERE client_id = $1")
        .bind(&client_id)
        .execute(&pool)
        .await
        .expect("oauth client revoke should succeed");

    let app = drive_app().await;
    let response = drive_objects_response(&app, workspace_id, &access_token).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response_error_code(response).await, "invalid_token");

    cleanup(&pool, tenant_id, &owner_email).await;
}

#[tokio::test]
async fn http_suspended_service_account_is_rejected_by_drive_workspace_route() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    if !db_supports_current_schema(&pool).await {
        eprintln!("skipping test: local database is missing current identity/drive schema columns");
        return;
    }

    let identity_state = identity_state(&pool).await;
    let identity_base_url = spawn_identity_server(identity_state).await;
    let (tenant_id, workspace_id, client_id, client_secret, principal_id, owner_email) =
        seed_machine_workspace_context(&pool).await;

    unsafe {
        std::env::set_var("NVBES_IDENTITY_BASE_URL", &identity_base_url);
        std::env::set_var("NVBES_IDENTITY_CLIENT_ID", &client_id);
        std::env::set_var("NVBES_IDENTITY_CLIENT_SECRET", &client_secret);
    }

    let access_token = issue_machine_token(&identity_base_url, &client_id, &client_secret).await;

    sqlx::query("UPDATE principals SET status = 'suspended', updated_at = NOW() WHERE id = $1")
        .bind(principal_id)
        .execute(&pool)
        .await
        .expect("service account suspend should succeed");

    let app = drive_app().await;
    let response = drive_objects_response(&app, workspace_id, &access_token).await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(response_error_code(response).await, "invalid_token");

    cleanup(&pool, tenant_id, &owner_email).await;
}

#[tokio::test]
async fn http_workspace_mismatch_is_rejected_by_drive_workspace_route() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    if !db_supports_current_schema(&pool).await {
        eprintln!("skipping test: local database is missing current identity/drive schema columns");
        return;
    }

    let identity_state = identity_state(&pool).await;
    let identity_base_url = spawn_identity_server(identity_state).await;
    let (tenant_id, _workspace_id, client_id, client_secret, _principal_id, owner_email) =
        seed_machine_workspace_context(&pool).await;

    unsafe {
        std::env::set_var("NVBES_IDENTITY_BASE_URL", &identity_base_url);
        std::env::set_var("NVBES_IDENTITY_CLIENT_ID", &client_id);
        std::env::set_var("NVBES_IDENTITY_CLIENT_SECRET", &client_secret);
    }

    let access_token = issue_machine_token(&identity_base_url, &client_id, &client_secret).await;
    let app = drive_app().await;
    let other_workspace_id = Uuid::new_v4();
    let response = drive_objects_response(&app, other_workspace_id, &access_token).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response_error_code(response).await,
        "workspace_context_mismatch"
    );

    cleanup(&pool, tenant_id, &owner_email).await;
}
