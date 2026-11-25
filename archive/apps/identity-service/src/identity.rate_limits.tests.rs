use super::*;
use crate::{
    oauth::{
        codes, maintenance,
        store::{self, RequestKind},
    },
    test_fixtures::{authorize, clients, database, exchange, request, session},
};

#[tokio::test]
async fn fixed_slots_limit_concurrent_attempts_and_reset_only_after_expiry() {
    let db = database().await;
    let limiter = RateLimiter::new([51; 32]).unwrap();
    let subject = uuid::Uuid::new_v4().to_string();
    let slot = limiter.slot("login_account", &subject);
    sqlx::query("DELETE FROM identity_rate_buckets WHERE category='login_account' AND slot=$1")
        .bind(slot)
        .execute(&db)
        .await
        .unwrap();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..20 {
        let (db, limiter, subject) = (db.clone(), limiter.clone(), subject.clone());
        tasks.spawn(async move { limiter.check(&db, Category::LoginAccount, &subject).await });
    }
    let mut admitted = 0;
    while let Some(result) = tasks.join_next().await {
        match result.unwrap() {
            Ok(()) => admitted += 1,
            Err(LimitError::Exceeded) => (),
            Err(error) => panic!("unexpected limiter failure: {error}"),
        }
    }
    assert_eq!(admitted, 5);
    sqlx::query("UPDATE identity_rate_buckets SET reset_at=clock_timestamp()-interval '1 second' WHERE category='login_account' AND slot=$1").bind(slot).execute(&db).await.unwrap();
    assert!(
        limiter
            .check(&db, Category::LoginAccount, &subject)
            .await
            .is_ok()
    );
    assert!(RateLimiter::new([0; 32]).is_err());
    assert!(matches!(
        limiter.check(&db, Category::LoginAccount, "").await,
        Err(LimitError::Configuration)
    ));
    // The physical constraint holds independently of the Rust caller.
    assert!(sqlx::query("INSERT INTO identity_rate_buckets(category,slot,attempts,reset_at) VALUES('login_source',4096,1,clock_timestamp())").execute(&db).await.is_err());
}

#[tokio::test]
async fn cleanup_is_bounded_skips_locked_requests_and_preserves_replay_evidence() {
    let db = database().await;
    let clients = clients();
    let handle = request(&db, &clients, RequestKind::Authorization).await;
    let (_, session) = session(&db).await;
    let code = authorize(&db, &clients, &handle, &session).await.unwrap();
    let grant = codes::exchange(&db, &clients, exchange(&code.code))
        .await
        .unwrap();
    sqlx::query("UPDATE identity_oauth_codes SET created_at=clock_timestamp()-interval '10 minutes',expires_at=clock_timestamp()-interval '5 minutes' WHERE code_hash=$1")
        .bind(store::hash(&code.code)).execute(&db).await.unwrap();
    let hashes: Vec<Vec<u8>> = (0..70)
        .map(|_| store::hash(&store::random_secret()))
        .collect();
    sqlx::query("INSERT INTO identity_oauth_requests(handle_hash,kind,client_id,parameters,created_at,expires_at) SELECT hash,'authorization','account-web','{}'::jsonb,clock_timestamp()-interval '10 minutes',clock_timestamp()-interval '5 minutes' FROM unnest($1::bytea[]) AS hash")
        .bind(&hashes).execute(&db).await.unwrap();
    let mut lock = db.begin().await.unwrap();
    sqlx::query("SELECT handle_hash FROM identity_oauth_requests WHERE handle_hash=$1 FOR UPDATE")
        .bind(&hashes[0])
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        maintenance::cleanup_expired(&db),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(result.requests <= 64 && result.unexchanged_codes <= 64 && result.consents <= 64);
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_oauth_requests WHERE handle_hash=ANY($1)",
    )
    .bind(&hashes)
    .fetch_one(&db)
    .await
    .unwrap();
    assert!(remaining >= 6);
    lock.rollback().await.unwrap();
    assert!(
        codes::exchange(&db, &clients, exchange(&code.code))
            .await
            .is_err()
    );
    let revoked: bool =
        sqlx::query_scalar("SELECT revoked_at IS NOT NULL FROM identity_oauth_grants WHERE id=$1")
            .bind(grant.id())
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(
        revoked,
        "cleanup must preserve consumed codes while their grants exist"
    );
}
