use std::net::SocketAddr;

use axum::{
    body::{self, Body},
    extract::connect_info::ConnectInfo,
    http::{Request, StatusCode},
};
use sqlx::postgres::PgPool;
use tower::ServiceExt;

#[path = "drive.domains.auth.e2e.support.rs"]
mod support;
use support::*;

async fn public_api_me_response(
    app: &axum::Router,
    access_token: &str,
    forwarded_ip: Option<&str>,
) -> axum::response::Response {
    let mut builder = Request::builder()
        .uri("/v1/me")
        .header("authorization", format!("Bearer {access_token}"))
        .header("user-agent", "drive-public-api-e2e")
        .extension(ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 49152))));

    if let Some(ip) = forwarded_ip {
        builder = builder.header("x-forwarded-for", ip);
    }

    app.clone()
        .oneshot(builder.body(Body::empty()).expect("request should build"))
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
async fn http_m2m_token_authorizes_public_api_me() {
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
    let app = drive_app().await;
    let response = public_api_me_response(&app, &access_token, Some("198.51.100.42")).await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body should be readable");
    let payload: serde_json::Value =
        serde_json::from_slice(&body).expect("response body should be valid json");
    assert_eq!(payload["workspace_id"], workspace_id.to_string());
    assert_eq!(payload["key_prefix"], "m2m");
    assert!(
        payload["scopes"]
            .as_array()
            .is_some_and(|scopes| scopes.iter().any(|scope| scope == "drive.files.read"))
    );

    cleanup(&pool, tenant_id, &owner_email).await;
}

#[tokio::test]
async fn http_public_api_auth_rejects_m2m_token_from_blocked_network() {
    let _guard = test_lock().lock().await;
    let pool = PgPool::connect_lazy(&test_database_url()).expect("valid pool");
    if !db_supports_current_schema(&pool).await {
        eprintln!("skipping test: local database is missing current identity/drive schema columns");
        return;
    }

    seed_public_api_network_range(&pool, "8.8.7.0/24", "vpn", 95, &["vpn", "anonymous"]).await;
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
    let response = public_api_me_response(&app, &access_token, Some("8.8.7.42")).await;

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response_error_code(response).await, "network_risk_blocked");

    cleanup(&pool, tenant_id, &owner_email).await;
}
