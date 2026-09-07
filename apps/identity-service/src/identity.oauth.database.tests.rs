use super::{
    codes,
    store::{self, RequestKind},
};
use crate::test_fixtures::{clients, database, exchange, request, session};

#[tokio::test]
async fn par_is_bound_to_client_and_consumed_once_under_concurrency() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Par).await;
    assert!(
        store::consume_par(&db, &clients, &handle, "other-client")
            .await
            .is_err()
    );
    let (a, b) = tokio::join!(
        store::consume_par(&db, &clients, &handle, "account-web"),
        store::consume_par(&db, &clients, &handle, "account-web")
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let next = a.or(b).unwrap();
    assert!(store::load_request(&db, &clients, &next).await.is_ok());
    let raw_stored: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM identity_oauth_requests WHERE handle_hash=convert_to($1,'UTF8'))")
        .bind(&next).fetch_one(&db).await.unwrap();
    assert!(!raw_stored);
}

#[tokio::test]
async fn exchange_binds_pkce_client_redirect_and_revokes_grant_on_valid_replay() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (_, token) = session(&db).await;
    let code = codes::authorize(&db, &clients, &handle, &token)
        .await
        .unwrap()
        .code;
    assert!(
        codes::authorize(&db, &clients, &handle, &token)
            .await
            .is_err()
    );
    let mut bad = exchange(&code);
    bad.verifier = "invalid";
    assert!(codes::exchange(&db, &clients, bad).await.is_err());
    let mut bad = exchange(&code);
    bad.redirect_uri = "https://evil.example/";
    assert!(codes::exchange(&db, &clients, bad).await.is_err());
    let mut bad = exchange(&code);
    bad.client_id = "other-client";
    assert!(codes::exchange(&db, &clients, bad).await.is_err());
    let grant = codes::exchange(&db, &clients, exchange(&code))
        .await
        .unwrap();
    sqlx::query("UPDATE identity_oauth_codes SET created_at=clock_timestamp()-interval '10 minutes',expires_at=clock_timestamp()-interval '5 minutes' WHERE code_hash=$1")
        .bind(store::hash(&code)).execute(&db).await.unwrap();
    assert!(
        codes::exchange(&db, &clients, exchange(&code))
            .await
            .is_err()
    );
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity_oauth_grants WHERE id=$1")
            .bind(grant.id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(revoked);
}

#[tokio::test]
async fn expired_requests_and_revoked_sessions_cannot_authorize() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    sqlx::query("UPDATE identity_oauth_requests SET created_at=clock_timestamp()-interval '10 minutes',expires_at=clock_timestamp()-interval '5 minutes' WHERE handle_hash=$1")
        .bind(store::hash(&handle)).execute(&db).await.unwrap();
    assert!(store::load_request(&db, &clients, &handle).await.is_err());
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (id, token) = session(&db).await;
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(id)
        .execute(&db)
        .await
        .unwrap();
    assert!(
        codes::authorize(&db, &clients, &handle, &token)
            .await
            .is_err()
    );
    assert!(store::load_request(&db, &clients, &handle).await.is_ok());
}

#[tokio::test]
async fn concurrent_code_exchanges_issue_only_one_grant() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (_, token) = session(&db).await;
    let code = codes::authorize(&db, &clients, &handle, &token)
        .await
        .unwrap()
        .code;
    let (a, b) = tokio::join!(
        codes::exchange(&db, &clients, exchange(&code)),
        codes::exchange(&db, &clients, exchange(&code))
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let grant = a.or(b).unwrap();
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity_oauth_grants WHERE id=$1")
            .bind(grant.id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(
        revoked,
        "a valid duplicate exchange revokes the single resulting grant"
    );
}
