use super::*;
use crate::domains::oauth::service::ClientAuthentication;
use axum::{Json, extract::State, http::HeaderMap};

#[path = "identity.domains.oauth.flows.client_credentials.tests.rotation.rs"]
mod rotation;
#[path = "identity.domains.oauth.flows.client_credentials.tests.seed.rs"]
mod seed;
#[path = "identity.domains.oauth.flows.client_credentials.tests.state.rs"]
mod state_support;

use seed::{cleanup, seed_service_client};
use state_support::{basic_auth_header, db_supports_current_oauth_schema, test_pool, test_state};

#[test]
fn machine_token_audit_metadata_records_grant_client_scope_audience_and_jti() {
    let metadata = super::machine_token_audit_metadata(
        "gxoc_machine",
        "access-jti-1",
        "drive.files.read drive.workspace.read",
        "nvbes-cloud-service",
    );

    assert_eq!(metadata["grant_type"], "client_credentials");
    assert_eq!(metadata["client_id"], "gxoc_machine");
    assert_eq!(metadata["jti"], "access-jti-1");
    assert_eq!(metadata["scope"], "drive.files.read drive.workspace.read");
    assert_eq!(metadata["audience"], "nvbes-cloud-service");
}

#[tokio::test]
async fn client_credentials_token_introspection_exposes_service_account_context() {
    let pool = test_pool();
    if !db_supports_current_oauth_schema(&pool).await {
        eprintln!("skipping test: local database is missing recent oauth schema migrations");
        return;
    }
    let state = test_state(&pool).await;
    let (tenant_id, client_id, client_secret, principal_id, workspace_id, _) =
        seed_service_client(&pool).await;

    let token = client_credentials_grant(
        &state.db,
        &state.jwt,
        ClientAuthentication {
            client_id: client_id.clone(),
            client_secret: Some(client_secret.clone()),
            client_assertion: None,
            client_assertion_verified: false,
        },
        Some("drive.files.read drive.workspace.read"),
        None,
    )
    .await
    .expect("client_credentials should succeed");

    let introspection = crate::domains::oauth::flows::introspect_token(
        &state.db,
        &state.redis,
        &state.jwt,
        &client_id,
        &token.access_token,
        Some("access_token".to_string()),
        None,
    )
    .await
    .expect("introspection should succeed");

    assert!(introspection.active);
    assert_eq!(
        introspection.principal_type.as_deref(),
        Some("service_account")
    );
    let expected_subject = principal_id.to_string();
    assert_eq!(
        introspection.sub.as_deref(),
        Some(expected_subject.as_str())
    );
    assert_eq!(introspection.workspace_id, Some(workspace_id));
    assert_eq!(introspection.tenant_id, Some(tenant_id));
    assert_eq!(introspection.role.as_deref(), Some("member"));
    assert_eq!(introspection.client_id.as_deref(), Some(client_id.as_str()));
    assert!(introspection.amr.iter().any(|method| method == "m2m"));

    cleanup(&pool, tenant_id).await;
}

#[tokio::test]
async fn client_credentials_records_last_used_and_machine_token_audit() {
    let pool = test_pool();
    let state = test_state(&pool).await;
    if !db_supports_current_oauth_schema(&pool).await {
        eprintln!("skipping test: local database is missing recent oauth schema migrations");
        return;
    }
    let (tenant_id, client_id, client_secret, principal_id, workspace_id, client_uuid) =
        seed_service_client(&pool).await;

    let token = client_credentials_grant(
        &state.db,
        &state.jwt,
        ClientAuthentication {
            client_id: client_id.clone(),
            client_secret: Some(client_secret),
            client_assertion: None,
            client_assertion_verified: false,
        },
        Some("drive.files.read"),
        None,
    )
    .await
    .expect("client_credentials should succeed");
    let claims = state
        .jwt
        .decode_token(&token.access_token, "access")
        .expect("machine token should decode");

    let last_used_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT last_used_at FROM oauth_clients WHERE id = $1")
            .bind(client_uuid)
            .fetch_one(&pool)
            .await
            .expect("client usage lookup should work");
    assert!(last_used_at.is_some());

    let audit_count = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COUNT(*)
        FROM audit_events
        WHERE tenant_id = $1
          AND workspace_id = $2
          AND actor_principal_id = $3
          AND action = 'oauth.machine_token.issued'
          AND target_type = 'oauth_client'
          AND target_id = $4
          AND metadata->>'client_id' = $5
          AND metadata->>'jti' = $6
          AND metadata->>'grant_type' = 'client_credentials'
        "#,
    )
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(principal_id)
    .bind(client_uuid)
    .bind(&client_id)
    .bind(&claims.jti)
    .fetch_one(&pool)
    .await
    .expect("machine token audit lookup should work");
    assert_eq!(audit_count, 1);

    cleanup(&pool, tenant_id).await;
}

#[tokio::test]
async fn introspection_rejects_machine_token_after_client_revocation() {
    let pool = test_pool();
    let state = test_state(&pool).await;
    if !db_supports_current_oauth_schema(&pool).await {
        eprintln!("skipping test: local database is missing recent oauth schema migrations");
        return;
    }
    let (tenant_id, client_id, client_secret, _, _, client_uuid) = seed_service_client(&pool).await;

    let token = client_credentials_grant(
        &state.db,
        &state.jwt,
        ClientAuthentication {
            client_id: client_id.clone(),
            client_secret: Some(client_secret),
            client_assertion: None,
            client_assertion_verified: false,
        },
        Some("drive.files.read"),
        None,
    )
    .await
    .expect("client_credentials should succeed");

    sqlx::query("UPDATE oauth_clients SET revoked_at = NOW() WHERE id = $1")
        .bind(client_uuid)
        .execute(&pool)
        .await
        .expect("client revocation should work");

    let introspection = crate::domains::oauth::flows::introspect_token(
        &state.db,
        &state.redis,
        &state.jwt,
        &client_id,
        &token.access_token,
        Some("access_token".to_string()),
        None,
    )
    .await
    .expect("introspection should respond");

    assert!(!introspection.active);

    cleanup(&pool, tenant_id).await;
}

#[tokio::test]
async fn http_client_credentials_and_introspection_expose_service_account_context() {
    let pool = test_pool();
    if !db_supports_current_oauth_schema(&pool).await {
        eprintln!("skipping test: local database is missing recent oauth schema migrations");
        return;
    }

    let state = test_state(&pool).await;
    let (tenant_id, client_id, client_secret, principal_id, workspace_id, _) =
        seed_service_client(&pool).await;
    let token_response = crate::domains::oauth::routes::token::token(
        State(state.clone()),
        basic_headers(&client_id, &client_secret),
        axum::Form(crate::domains::oauth::routes::token::TokenRequest {
            grant_type: "client_credentials".to_string(),
            code: None,
            refresh_token: None,
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            code_verifier: None,
            device_code: None,
            scope: Some("drive.files.read drive.workspace.read".to_string()),
            audience: Some("nvbes-cloud-service".to_string()),
            subject_token: None,
            subject_token_type: None,
            actor_token: None,
            actor_token_type: None,
            requested_token_type: None,
            client_assertion_type: None,
            client_assertion: None,
        }),
    )
    .await
    .expect("token endpoint should respond");

    let token_body = serde_json::to_value(token_response.0).expect("token body should serialize");
    let access_token = token_body["access_token"]
        .as_str()
        .expect("token response should include access_token");

    let introspection_response = crate::domains::oauth::routes::introspect::introspect(
        State(state),
        basic_headers(&client_id, &client_secret),
        Json(
            crate::domains::oauth::routes::introspect::IntrospectRequest {
                token: access_token.to_string(),
                token_type_hint: Some("access_token".to_string()),
            },
        ),
    )
    .await
    .expect("introspection endpoint should respond");

    let introspection =
        serde_json::to_value(introspection_response.0).expect("introspection should serialize");

    assert_eq!(introspection["active"], true);
    assert_eq!(introspection["principal_type"], "service_account");
    assert_eq!(introspection["sub"], principal_id.to_string());
    assert_eq!(introspection["workspace_id"], workspace_id.to_string());
    assert_eq!(introspection["tenant_id"], tenant_id.to_string());
    assert_eq!(introspection["role"], "member");
    assert_eq!(introspection["client_id"], client_id);
    assert_eq!(introspection["amr"], serde_json::json!(["m2m"]));

    cleanup(&pool, tenant_id).await;
}

fn basic_headers(client_id: &str, client_secret: &str) -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::AUTHORIZATION,
        basic_auth_header(client_id, client_secret)
            .parse()
            .expect("authorization header should parse"),
    );
    headers
}
