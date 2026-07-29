use super::*;
use crate::domains::oauth::service::{ClientAuthentication, TokenExchangeInput};
use axum::{extract::State, http::HeaderMap};
use chrono::Utc;
use uuid::Uuid;

#[path = "identity.domains.oauth.flows.token_exchange.tests.seed.rs"]
mod seed;
#[path = "identity.domains.oauth.flows.token_exchange.tests.state.rs"]
mod state_support;

use seed::{cleanup, seed_exchange_context};
use state_support::{assert_current_oauth_schema, basic_auth_header, test_pool, test_state};

#[tokio::test]
async fn token_exchange_preserves_user_subject_and_sets_machine_actor() {
    let pool = test_pool();
    assert_current_oauth_schema(&pool).await;

    let state = test_state(&pool).await;
    let (
        tenant_id,
        user_principal_id,
        service_principal_id,
        workspace_id,
        client_id,
        client_secret,
    ) = seed_exchange_context(&pool).await;

    let session_id = Uuid::new_v4();
    let subject_pair = state
        .jwt
        .generate_token_pair_with_session(
            user_principal_id,
            Some(workspace_id),
            None,
            "drive.files.read drive.workspace.read",
            Some(session_id),
            Some(tenant_id),
            None,
            Some("aal1"),
            Some(vec!["pwd".to_string()]),
            None,
            Some(Utc::now().timestamp()),
            None,
        )
        .await
        .expect("subject token should be created");

    let actor = crate::domains::oauth::flows::client_credentials_grant(
        &state.db,
        &state.jwt,
        ClientAuthentication {
            client_id: client_id.clone(),
            client_secret: Some(client_secret.clone()),
            client_assertion: None,
            client_assertion_verified: false,
        },
        Some("drive.files.read"),
        None,
        None,
    )
    .await
    .expect("actor token should be created");

    let exchanged = token_exchange(
        &state.db,
        &state.redis,
        &state.jwt,
        TokenExchangeInput {
            subject_token: subject_pair.access_token,
            subject_token_type: "urn:ietf:params:oauth:token-type:access_token".to_string(),
            actor_token: Some(actor.access_token),
            actor_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
            client_id: client_id.clone(),
            client_secret: Some(client_secret),
            client_assertion_verified: false,
            scope: Some("drive.files.read".to_string()),
            audience: None,
            resource: None,
            requested_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
        },
        None,
    )
    .await
    .expect("token exchange should succeed");

    let exchanged_claims = state
        .jwt
        .decode_token(&exchanged.access_token, "access")
        .expect("exchanged token should decode");

    let expected_subject = user_principal_id.to_string();
    let expected_workspace = workspace_id.to_string();
    let expected_tenant = tenant_id.to_string();
    let expected_actor = service_principal_id.to_string();

    assert_eq!(exchanged.scope, "drive.files.read");
    assert_eq!(exchanged_claims.sub, expected_subject);
    assert_eq!(
        exchanged_claims.workspace_id.as_deref(),
        Some(expected_workspace.as_str())
    );
    assert_eq!(
        exchanged_claims.tenant_id.as_deref(),
        Some(expected_tenant.as_str())
    );
    assert_eq!(
        exchanged_claims
            .act
            .as_ref()
            .map(|actor| actor.sub.as_str()),
        Some(expected_actor.as_str())
    );
    assert_eq!(
        exchanged_claims
            .act
            .as_ref()
            .and_then(|actor| actor.client_id.as_deref()),
        Some(client_id.as_str())
    );

    cleanup(&pool, tenant_id).await;
}

#[tokio::test]
async fn http_token_exchange_preserves_user_subject_and_sets_machine_actor() {
    let pool = test_pool();
    assert_current_oauth_schema(&pool).await;

    let state = test_state(&pool).await;
    let (
        tenant_id,
        user_principal_id,
        service_principal_id,
        workspace_id,
        client_id,
        client_secret,
    ) = seed_exchange_context(&pool).await;
    let session_id = Uuid::new_v4();
    let subject_pair = state
        .jwt
        .generate_token_pair_with_session(
            user_principal_id,
            Some(workspace_id),
            None,
            "drive.files.read drive.workspace.read",
            Some(session_id),
            Some(tenant_id),
            None,
            Some("aal1"),
            Some(vec!["pwd".to_string()]),
            None,
            Some(Utc::now().timestamp()),
            None,
        )
        .await
        .expect("subject token should be created");

    let actor_response = crate::domains::oauth::routes::token::token(
        State(state.clone()),
        basic_headers(&client_id, &client_secret),
        None,
        None,
        axum::Form(crate::domains::oauth::routes::token::TokenRequest {
            grant_type: "client_credentials".to_string(),
            code: None,
            refresh_token: None,
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            code_verifier: None,
            device_code: None,
            scope: Some("drive.files.read".to_string()),
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
    .expect("actor token endpoint should respond");

    let actor_body = serde_json::to_value(actor_response.0).expect("actor token should serialize");
    let actor_token = actor_body["access_token"]
        .as_str()
        .expect("actor token should be present")
        .to_string();

    let exchange_response = crate::domains::oauth::routes::token::token(
        State(state.clone()),
        basic_headers(&client_id, &client_secret),
        None,
        None,
        axum::Form(crate::domains::oauth::routes::token::TokenRequest {
            grant_type: "urn:ietf:params:oauth:grant-type:token-exchange".to_string(),
            code: None,
            refresh_token: None,
            client_id: None,
            client_secret: None,
            redirect_uri: None,
            code_verifier: None,
            device_code: None,
            scope: Some("drive.files.read".to_string()),
            audience: Some("nvbes-cloud-service".to_string()),
            subject_token: Some(subject_pair.access_token),
            subject_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
            actor_token: Some(actor_token),
            actor_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
            requested_token_type: Some("urn:ietf:params:oauth:token-type:access_token".to_string()),
            client_assertion_type: None,
            client_assertion: None,
        }),
    )
    .await
    .expect("token exchange endpoint should respond");

    let exchange_body =
        serde_json::to_value(exchange_response.0).expect("exchange token should serialize");
    let exchanged_token = exchange_body["access_token"]
        .as_str()
        .expect("exchanged token should be present");

    let exchanged_claims = state
        .jwt
        .decode_token(exchanged_token, "access")
        .expect("exchanged token should decode");

    assert_eq!(exchanged_claims.sub, user_principal_id.to_string());
    assert_eq!(
        exchanged_claims.workspace_id.as_deref(),
        Some(workspace_id.to_string().as_str())
    );
    assert_eq!(
        exchanged_claims.tenant_id.as_deref(),
        Some(tenant_id.to_string().as_str())
    );
    assert_eq!(
        exchanged_claims
            .act
            .as_ref()
            .map(|actor| actor.sub.as_str()),
        Some(service_principal_id.to_string().as_str())
    );
    assert_eq!(
        exchanged_claims
            .act
            .as_ref()
            .and_then(|actor| actor.client_id.as_deref()),
        Some(client_id.as_str())
    );

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
