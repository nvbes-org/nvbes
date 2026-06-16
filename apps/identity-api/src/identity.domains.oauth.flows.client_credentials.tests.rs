use super::*;
use crate::domains::oauth::service::ClientAuthentication;
use axum::{Json, extract::State, http::HeaderMap};

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
        "nvbes-drive-api",
    );

    assert_eq!(metadata["grant_type"], "client_credentials");
    assert_eq!(metadata["client_id"], "gxoc_machine");
    assert_eq!(metadata["jti"], "access-jti-1");
    assert_eq!(metadata["scope"], "drive.files.read drive.workspace.read");
    assert_eq!(metadata["audience"], "nvbes-drive-api");
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
            audience: Some("nvbes-drive-api".to_string()),
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

#[tokio::test]
async fn test_client_credentials_grant_with_rotated_secret_overlap() {
    let pool = test_pool();
    if !db_supports_current_oauth_schema(&pool).await {
        eprintln!("skipping test: local database is missing recent oauth schema migrations");
        return;
    }
    let state = test_state(&pool).await;
    let (tenant_id, client_id, client_secret, _principal_id, _workspace_id, _client_uuid) =
        seed_service_client(&pool).await;

    // 1. Initial authentication with client_secret should succeed
    let token1 = client_credentials_grant(
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
    .await;
    assert!(token1.is_ok(), "Initial authentication should succeed");

    // 2. Perform rotation using the developer route's logic or custom DB inserts.
    // Let's create an overlap version for the old client_secret.
    let overlap_ends_at = chrono::Utc::now() + chrono::Duration::hours(24);
    let old_secret_hash = crate::domains::oauth::hash_client_secret(&client_secret).unwrap();

    let mut tx = state.db.begin().await.unwrap();
    let previous_version_id =
        crate::domains::developer::service_accounts_db::ensure_previous_overlap_version(
            &mut tx,
            tenant_id,
            &client_id,
            &old_secret_hash,
            overlap_ends_at,
        )
        .await
        .unwrap();

    // Generate new secret
    let new_client_secret = "gxo_new_secret_1234567890_value";
    let new_client_secret_hash =
        crate::domains::oauth::hash_client_secret(new_client_secret).unwrap();
    let secret_last4 =
        crate::domains::developer::service_accounts_db::secret_last4(new_client_secret);

    // Insert new active secret version
    let _active_version_id: uuid::Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO developer_client_secret_versions (
          tenant_id,
          client_id,
          status,
          client_secret_hash,
          secret_last4,
          created_at
        )
        VALUES ($1, $2, 'active', $3, $4, NOW())
        RETURNING id
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(&new_client_secret_hash)
    .bind(secret_last4)
    .fetch_one(&mut *tx)
    .await
    .unwrap();

    // Update main client secret in oauth_clients
    sqlx::query(
        "UPDATE oauth_clients SET client_secret_hash = $3, updated_at = NOW() WHERE tenant_id = $1 AND client_id = $2",
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(&new_client_secret_hash)
    .execute(&mut *tx)
    .await
    .unwrap();

    tx.commit().await.unwrap();

    // 3. Authenticating with the NEW secret should succeed
    let token_new = client_credentials_grant(
        &state.db,
        &state.jwt,
        ClientAuthentication {
            client_id: client_id.clone(),
            client_secret: Some(new_client_secret.to_string()),
            client_assertion: None,
            client_assertion_verified: false,
        },
        Some("drive.files.read drive.workspace.read"),
        None,
    )
    .await;
    assert!(
        token_new.is_ok(),
        "New secret authentication should succeed"
    );

    // 4. Authenticating with the OLD secret during overlap period should succeed
    let token_old = client_credentials_grant(
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
    .await;
    assert!(
        token_old.is_ok(),
        "Old secret authentication should succeed during overlap"
    );

    // 5. If we revoke the previous secret version, old secret authentication should fail
    sqlx::query(
        r#"
        UPDATE developer_client_secret_versions
        SET status = 'revoked',
            revoked_at = NOW()
        WHERE tenant_id = $1
          AND client_id = $2
          AND id = $3
        "#,
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(previous_version_id)
    .execute(&state.db)
    .await
    .unwrap();

    let token_revoked = client_credentials_grant(
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
    .await;
    assert!(
        token_revoked.is_err(),
        "Old secret authentication should fail after revocation"
    );

    cleanup(&pool, tenant_id).await;
}
