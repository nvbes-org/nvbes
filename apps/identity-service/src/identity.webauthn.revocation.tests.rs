use super::*;
use crate::{
    oauth::{codes, store::RequestKind},
    test_fixtures as fixtures,
    tokens::{RefreshRequest, TokenService},
};

#[tokio::test]
async fn revoking_a_used_key_ends_sessions_and_disables_grants_and_refresh() {
    let f = Fixture::new(2).await;
    let clients = fixtures::clients();
    let service = TokenService::new(crate::tokens::tests::config()).unwrap();
    let mut input = fixtures::input();
    input.scope.push_str(" offline_access");
    let handle = store::create_request(
        &f.db,
        &input.validate(&clients).unwrap(),
        RequestKind::Authorization,
    )
    .await
    .unwrap();
    let code = fixtures::authorize(&f.db, &clients, &handle, &f.token)
        .await
        .unwrap();
    let grant = codes::exchange(&f.db, &clients, fixtures::exchange(&code.code))
        .await
        .unwrap();
    let tokens = service.issue_grant(&f.db, &clients, &grant).await.unwrap();
    let audience = "nvbes-account-service";
    assert!(
        service
            .introspect(&f.db, &clients, &tokens.access_token, audience)
            .await
            .unwrap()
            .is_some()
    );
    let pending = fixtures::request(&f.db, &clients, RequestKind::Authorization).await;
    let pending_code = fixtures::authorize(&f.db, &clients, &pending, &f.token)
        .await
        .unwrap();
    let (unrelated, unrelated_token) = session(&f.db).await;
    sqlx::query("UPDATE identity_sessions SET principal_id=(SELECT principal_id FROM identity_sessions WHERE id=$1) WHERE id=$2")
        .bind(f.session).bind(unrelated).execute(&f.db).await.unwrap();
    revoke(&f.db, &f.token, f.ids[1]).await.unwrap();
    assert!(matches!(
        list(&f.db, &f.token).await,
        Err(WebauthnError::InvalidSession)
    ));
    assert!(list(&f.db, &unrelated_token).await.is_ok());
    assert!(
        codes::exchange(&f.db, &clients, fixtures::exchange(&pending_code.code))
            .await
            .is_err()
    );
    assert!(
        service
            .introspect(&f.db, &clients, &tokens.access_token, audience)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        service
            .refresh(
                &f.db,
                &clients,
                RefreshRequest {
                    refresh_token: tokens.refresh_token.as_deref().unwrap(),
                    client_id: "account-web",
                    dpop_proof: None
                }
            )
            .await
            .is_err()
    );
    let active: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_sessions s JOIN identity_session_webauthn_credentials b ON b.session_id=s.id WHERE b.credential_id=$1 AND s.revoked_at IS NULL")
        .bind(f.ids[1]).fetch_one(&f.db).await.unwrap();
    assert_eq!(active, 0);
}

#[tokio::test]
async fn audit_failure_restores_both_credential_and_all_associated_sessions() {
    let f = Fixture::new(2).await;
    sqlx::query("ALTER TABLE identity_audit_events ADD CONSTRAINT injected_revocation_failure CHECK (event_type<>'identity.webauthn.revoked')")
        .execute(&f.db).await.unwrap();
    assert!(matches!(
        revoke(&f.db, &f.token, f.ids[1]).await,
        Err(WebauthnError::Database(_))
    ));
    assert!(list(&f.db, &f.token).await.is_ok());
    let revoked: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_sessions s JOIN identity_session_webauthn_credentials b ON b.session_id=s.id WHERE b.credential_id=$1 AND s.revoked_at IS NOT NULL")
        .bind(f.ids[1]).fetch_one(&f.db).await.unwrap();
    assert_eq!(revoked, 0);
    sqlx::query("ALTER TABLE identity_audit_events DROP CONSTRAINT injected_revocation_failure")
        .execute(&f.db)
        .await
        .unwrap();
    revoke(&f.db, &f.token, f.ids[1]).await.unwrap();
    assert!(matches!(
        list(&f.db, &f.token).await,
        Err(WebauthnError::InvalidSession)
    ));
}

#[tokio::test]
async fn migration_retires_unattributed_webauthn_sessions_once() {
    let f = Fixture::new(2).await;
    let (primary, _) = session(&f.db).await;
    let (step_up, _) = session(&f.db).await;
    let (password, _) = session(&f.db).await;
    let (legacy, _) = session(&f.db).await;
    sqlx::query("UPDATE identity_sessions SET created_at=(SELECT installed_on-interval '1 second' FROM _sqlx_migrations WHERE version=18) WHERE id=$1")
        .bind(legacy).execute(&f.db).await.unwrap();
    sqlx::query("UPDATE identity_sessions SET primary_amr='webauthn' WHERE id=$1")
        .bind(primary)
        .execute(&f.db)
        .await
        .unwrap();
    sqlx::query("UPDATE identity_sessions SET step_up_method='webauthn',step_up_at=clock_timestamp(),step_up_expires_at=expires_at WHERE id=$1")
        .bind(step_up).execute(&f.db).await.unwrap();
    let migration = include_str!("../migrations/0019_identity_webauthn_session_transition.sql");
    sqlx::raw_sql(migration).execute(&f.db).await.unwrap();
    sqlx::raw_sql(migration).execute(&f.db).await.unwrap();
    for id in [primary, step_up, legacy] {
        let revoked: bool =
            sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity_sessions WHERE id=$1")
                .bind(id)
                .fetch_one(&f.db)
                .await
                .unwrap();
        assert!(revoked);
    }
    for id in [password, f.session] {
        let revoked: bool =
            sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity_sessions WHERE id=$1")
                .bind(id)
                .fetch_one(&f.db)
                .await
                .unwrap();
        assert!(!revoked);
    }
    let audits: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_audit_events WHERE event_type='identity.sessions.webauthn_transition'")
        .fetch_one(&f.db).await.unwrap();
    assert_eq!(audits, 3);
}
