use super::*;

#[tokio::test]
async fn client_credentials_grant_accepts_rotated_primary_secret() {
    let pool = test_pool();
    if !db_supports_current_oauth_schema(&pool).await {
        eprintln!("skipping test: local database is missing recent oauth schema migrations");
        return;
    }
    let state = test_state(&pool).await;
    let (tenant_id, client_id, client_secret, _principal_id, _workspace_id, _client_uuid) =
        seed_service_client(&pool).await;

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
        None,
    )
    .await;
    assert!(token1.is_ok(), "Initial authentication should succeed");

    let new_client_secret = "gxo_new_secret_1234567890_value";
    let new_client_secret_hash =
        crate::domains::oauth::hash_client_secret(new_client_secret).unwrap();

    sqlx::query(
        "UPDATE oauth_clients SET client_secret_hash = $3, updated_at = NOW() WHERE tenant_id = $1 AND client_id = $2",
    )
    .bind(tenant_id)
    .bind(&client_id)
    .bind(&new_client_secret_hash)
    .execute(&state.db)
    .await
    .unwrap();

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
        None,
    )
    .await;
    assert!(
        token_new.is_ok(),
        "New secret authentication should succeed"
    );

    cleanup(&pool, tenant_id).await;
}
