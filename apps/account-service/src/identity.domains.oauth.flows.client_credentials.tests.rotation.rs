use super::*;

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

    let new_client_secret = "gxo_new_secret_1234567890_value";
    let new_client_secret_hash =
        crate::domains::oauth::hash_client_secret(new_client_secret).unwrap();
    let secret_last4 =
        crate::domains::developer::service_accounts_db::secret_last4(new_client_secret);

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
