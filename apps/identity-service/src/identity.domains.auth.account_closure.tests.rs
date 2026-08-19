use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use super::{AccountClosureCommand, EVENT_TYPE, apply, validate_command};

fn command(principal_id: Uuid) -> AccountClosureCommand {
    AccountClosureCommand {
        event_id: Uuid::new_v4(),
        event_type: EVENT_TYPE.to_string(),
        saga_id: Uuid::new_v4(),
        principal_id,
        requested_at: Utc::now(),
    }
}

#[test]
fn closure_command_requires_the_exact_contract_version() {
    let mut command = command(Uuid::new_v4());
    validate_command(&command).expect("current closure contract");
    command.event_type = "account.closure.requested".to_string();
    assert!(validate_command(&command).is_err());
}

#[tokio::test]
async fn closure_purges_credentials_and_is_replay_safe() {
    let db = crate::test_support::shared_test_pool();
    if !crate::test_support::is_test_database_available(&db).await {
        eprintln!("skipping test: database not available");
        return;
    }
    crate::test_support::ensure_test_database(&db).await;
    let (tenant_id, principal_id, session_id) = seed_subject(&db).await;
    let command = command(principal_id);

    assert!(apply(&db, &command).await.expect("first closure applies"));
    assert!(!apply(&db, &command).await.expect("exact replay converges"));

    let user = sqlx::query(
        "SELECT email, password_hash, status::text AS status FROM users WHERE principal_id = $1",
    )
    .bind(principal_id)
    .fetch_one(&db)
    .await
    .expect("user tombstone");
    assert_eq!(user.get::<String, _>("status"), "deleted");
    assert_eq!(
        user.get::<String, _>("email"),
        format!("closed-{principal_id}@deleted.invalid")
    );
    assert!(user.get::<Option<String>, _>("password_hash").is_none());

    for table in [
        "mfa_factors",
        "user_identities",
        "user_email_addresses",
        "oauth_consents",
        "account_devices",
        "identity_oidc_profile_claims",
        "password_history",
        "developer_role_assignments",
        "privileged_access_grants",
        "tenant_break_glass_accounts",
        "tenant_memberships",
    ] {
        let count: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM {table} WHERE principal_id = $1"
        ))
        .bind(principal_id)
        .fetch_one(&db)
        .await
        .expect("credential count");
        assert_eq!(count, 0, "{table} was not purged");
    }

    let session = sqlx::query(
        "SELECT revoked_at, browser_session_token_hash FROM user_sessions WHERE session_id = $1",
    )
    .bind(session_id)
    .fetch_one(&db)
    .await
    .expect("session tombstone");
    assert!(
        session
            .get::<Option<chrono::DateTime<Utc>>, _>("revoked_at")
            .is_some()
    );
    assert!(
        session
            .get::<Option<String>, _>("browser_session_token_hash")
            .is_none()
    );

    cleanup(&db, tenant_id, principal_id).await;
}

#[tokio::test]
async fn closure_rejects_event_identifier_collisions() {
    let db = crate::test_support::shared_test_pool();
    if !crate::test_support::is_test_database_available(&db).await {
        eprintln!("skipping test: database not available");
        return;
    }
    crate::test_support::ensure_test_database(&db).await;
    let (tenant_id, principal_id, _) = seed_subject(&db).await;
    let first = command(principal_id);
    apply(&db, &first).await.expect("first closure applies");
    let collision = AccountClosureCommand {
        saga_id: Uuid::new_v4(),
        ..first
    };
    assert!(apply(&db, &collision).await.is_err());
    cleanup(&db, tenant_id, principal_id).await;
}

async fn seed_subject(db: &PgPool) -> (Uuid, Uuid, Uuid) {
    let tenant_id = Uuid::new_v4();
    let principal_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO tenants (id, kind, name, slug, status)
        VALUES ($1, 'personal', 'Closure test', $2, 'active')
        "#,
    )
    .bind(tenant_id)
    .bind(format!("closure-{tenant_id}"))
    .execute(db)
    .await
    .expect("tenant");
    sqlx::query(
        r#"
        INSERT INTO principals (id, tenant_id, principal_kind, status, display_name)
        VALUES ($1, $2, 'human', 'active', 'Closure test')
        "#,
    )
    .bind(principal_id)
    .bind(tenant_id)
    .execute(db)
    .await
    .expect("principal");
    sqlx::query(
        r#"
        INSERT INTO users (principal_id, email, password_hash, email_verified_at, status)
        VALUES ($1, $2, 'password-hash', NOW(), 'active')
        "#,
    )
    .bind(principal_id)
    .bind(format!("closure-{principal_id}@example.test"))
    .execute(db)
    .await
    .expect("user");
    sqlx::query(
        r#"
        INSERT INTO tenant_memberships
          (tenant_id, principal_id, principal_kind, role, status, source)
        VALUES ($1, $2, 'human', 'member', 'active', 'manual')
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .execute(db)
    .await
    .expect("tenant membership");
    sqlx::query(
        r#"
        INSERT INTO developer_role_assignments (tenant_id, principal_id, role)
        VALUES ($1, $2, 'developer_admin')
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .execute(db)
    .await
    .expect("developer role");
    sqlx::query(
        r#"
        INSERT INTO privileged_access_grants
          (tenant_id, principal_id, granted_by, role, reason, expires_at)
        VALUES ($1, $2, $2, 'member', 'Closure test', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .execute(db)
    .await
    .expect("privileged grant");
    sqlx::query(
        r#"
        INSERT INTO tenant_break_glass_accounts
          (tenant_id, principal_id, procedure_reference, reason, created_by_principal_id)
        VALUES ($1, $2, 'TEST-1', 'Closure test', $2)
        "#,
    )
    .bind(tenant_id)
    .bind(principal_id)
    .execute(db)
    .await
    .expect("break-glass account");
    sqlx::query(
        r#"
        INSERT INTO mfa_factors (principal_id, factor_type, status, totp_secret_base32)
        VALUES ($1, 'totp', 'active', 'SECRET')
        "#,
    )
    .bind(principal_id)
    .execute(db)
    .await
    .expect("factor");
    sqlx::query(
        r#"
        INSERT INTO password_history (principal_id, password_hash)
        VALUES ($1, 'old-password-hash')
        "#,
    )
    .bind(principal_id)
    .execute(db)
    .await
    .expect("password history");
    sqlx::query(
        r#"
        INSERT INTO user_sessions (
          session_id, principal_id, browser_session_token_hash, expires_at
        )
        VALUES ($1, $2, 'browser-token-hash', NOW() + INTERVAL '1 hour')
        "#,
    )
    .bind(session_id)
    .bind(principal_id)
    .execute(db)
    .await
    .expect("session");
    sqlx::query(
        r#"
        INSERT INTO identity_oidc_profile_claims (principal_id, display_name)
        VALUES ($1, 'Closure test')
        "#,
    )
    .bind(principal_id)
    .execute(db)
    .await
    .expect("OIDC projection");
    (tenant_id, principal_id, session_id)
}

async fn cleanup(db: &PgPool, tenant_id: Uuid, principal_id: Uuid) {
    sqlx::query("DELETE FROM identity_inbox_events WHERE principal_id = $1")
        .bind(principal_id)
        .execute(db)
        .await
        .ok();
    sqlx::query("DELETE FROM principals WHERE id = $1")
        .bind(principal_id)
        .execute(db)
        .await
        .ok();
    sqlx::query("DELETE FROM tenants WHERE id = $1")
        .bind(tenant_id)
        .execute(db)
        .await
        .ok();
}
