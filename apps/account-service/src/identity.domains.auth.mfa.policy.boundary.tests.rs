use uuid::Uuid;

use super::principal_has_privileged_role;

#[tokio::test]
async fn privileged_workspace_role_protects_last_passkey_through_cloud_boundary() {
    let db = crate::test_support::shared_test_pool();
    crate::test_support::ensure_test_database(&db).await;
    let _guard = crate::test_support::test_database_lock().lock().await;
    crate::test_support::cloud_mock::ensure_cloud_mock(&db);

    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let workspace_id = Uuid::new_v4();
    seed_subject(&db, tenant_id, principal_id, workspace_id).await;
    let passkey_id = insert_passkey_and_totp(&db, principal_id).await;

    assert!(
        principal_has_privileged_role(&db, principal_id)
            .await
            .expect("Cloud workspace privilege lookup should succeed")
    );
    let error = crate::domains::auth::mfa::remove_factor(&db, principal_id, passkey_id)
        .await
        .expect_err("a privileged workspace owner must retain a passkey");
    assert_eq!(error.code, "privileged_passkey_required");
    assert_eq!(factor_status(&db, passkey_id).await, "active");

    sqlx::query(
        "UPDATE workspace_memberships SET role = 'member' WHERE workspace_id = $1 AND principal_id = $2",
    )
    .bind(workspace_id)
    .bind(principal_id)
    .execute(&db)
    .await
    .expect("workspace role should be downgraded");

    assert!(
        !principal_has_privileged_role(&db, principal_id)
            .await
            .expect("non-privileged Cloud workspace lookup should succeed")
    );
    crate::domains::auth::mfa::remove_factor(&db, principal_id, passkey_id)
        .await
        .expect("a non-privileged member may remove the passkey when another factor remains");
    assert_eq!(factor_status(&db, passkey_id).await, "revoked");

    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(&db)
        .await
        .expect("test tenant should be deleted");
}

async fn insert_passkey_and_totp(db: &sqlx::PgPool, principal_id: Uuid) -> Uuid {
    let passkey_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO mfa_factors (
          id, principal_id, factor_type, status, label, confirmed_at
        )
        VALUES
          ($1, $2, 'webauthn', 'active', 'Boundary passkey', NOW()),
          ($3, $2, 'totp', 'active', 'Boundary TOTP', NOW())
        "#,
    )
    .bind(passkey_id)
    .bind(principal_id)
    .bind(Uuid::new_v4())
    .execute(db)
    .await
    .expect("MFA factors should be inserted");
    passkey_id
}

async fn factor_status(db: &sqlx::PgPool, factor_id: Uuid) -> String {
    sqlx::query_scalar("SELECT status::text FROM mfa_factors WHERE id = $1")
        .bind(factor_id)
        .fetch_one(db)
        .await
        .expect("MFA factor status should be readable")
}

async fn seed_subject(db: &sqlx::PgPool, tenant_id: Uuid, principal_id: Uuid, workspace_id: Uuid) {
    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status, security_tier)
        VALUES ($1, 'personal', 'MFA Cloud boundary test', $2, 'active', 'standard')
        "#,
    )
    .bind(tenant_id)
    .bind(format!("mfa-cloud-boundary-{tenant_id}"))
    .execute(db)
    .await
    .expect("tenant should be inserted");
    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
        VALUES ($1, $2, 'human', 'active', 'MFA Cloud boundary user')
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .execute(db)
    .await
    .expect("principal should be inserted");
    sqlx::query(
        r#"
        INSERT INTO workspaces (id, tenant_id, name, workspace_type, plan_code)
        VALUES ($1, $2, 'MFA Cloud boundary workspace', 'team', 'solo_pro')
        "#,
    )
    .bind(workspace_id)
    .bind(tenant_id)
    .execute(db)
    .await
    .expect("workspace should be inserted");
    sqlx::query(
        r#"
        INSERT INTO workspace_policies (workspace_id)
        VALUES ($1)
        "#,
    )
    .bind(workspace_id)
    .execute(db)
    .await
    .expect("workspace policy should be inserted");
    sqlx::query(
        r#"
        INSERT INTO workspace_memberships (
          workspace_id, principal_id, role, status, source
        )
        VALUES ($1, $2, 'owner', 'active', 'manual')
        "#,
    )
    .bind(workspace_id)
    .bind(principal_id)
    .execute(db)
    .await
    .expect("workspace membership should be inserted");
}
