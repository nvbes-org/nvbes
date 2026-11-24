use super::*;
use crate::test_fixtures::{isolated_database, session};

fn guard(db: &PgPool) -> SessionContextGuard {
    SessionContextGuard::new(
        db.clone(),
        BrowserSecurity::new("http://localhost:3000", true).unwrap(),
        &RateLimiter::new([9; 32]).unwrap(),
    )
    .unwrap()
}

async fn flagged(db: &PgPool, id: uuid::Uuid) -> bool {
    sqlx::query_scalar("SELECT reauth_required_at IS NOT NULL FROM identity_sessions WHERE id=$1")
        .bind(id)
        .fetch_one(db)
        .await
        .unwrap()
}

async fn context_events(db: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM identity_audit_events WHERE event_type='identity.session.context_changed'",
    )
    .fetch_one(db)
    .await
    .unwrap()
}

#[tokio::test]
async fn first_presentation_binds_and_the_same_environment_stays_fresh() {
    let db = isolated_database().await;
    let (id, token) = session(&db).await;
    let guard = guard(&db);

    assert_eq!(
        guard.observe(&token, &[1; 32]).await.unwrap(),
        Observation::Bound
    );
    assert_eq!(
        guard.observe(&token, &[1; 32]).await.unwrap(),
        Observation::Unchanged
    );
    assert!(!flagged(&db, id).await);
    assert_eq!(context_events(&db).await, 0);
}

#[tokio::test]
async fn another_environment_requires_reauthentication_once_and_is_audited() {
    let db = isolated_database().await;
    let (id, token) = session(&db).await;
    let guard = guard(&db);
    guard.bind_issued(&token, &[1; 32]).await.unwrap();

    assert_eq!(
        guard.observe(&token, &[2; 32]).await.unwrap(),
        Observation::ReauthenticationRequired
    );
    assert!(flagged(&db, id).await);
    // Returning to the original environment does not clear the flag, and a
    // repeated mismatch is not audited twice.
    assert_eq!(
        guard.observe(&token, &[1; 32]).await.unwrap(),
        Observation::Unchanged
    );
    assert_eq!(
        guard.observe(&token, &[3; 32]).await.unwrap(),
        Observation::ReauthenticationRequired
    );
    assert!(flagged(&db, id).await);
    assert_eq!(context_events(&db).await, 1);
}

#[tokio::test]
async fn flagged_sessions_never_count_as_recently_authenticated() {
    let db = isolated_database().await;
    let (_, token) = session(&db).await;
    let guard = guard(&db);
    guard.bind_issued(&token, &[1; 32]).await.unwrap();

    let mut tx = db.begin().await.unwrap();
    let before = crate::authentication_session::load(&mut tx, &token)
        .await
        .unwrap()
        .unwrap();
    tx.rollback().await.unwrap();
    assert!(before.recent_primary);

    guard.observe(&token, &[2; 32]).await.unwrap();

    let mut tx = db.begin().await.unwrap();
    let after = crate::authentication_session::load(&mut tx, &token)
        .await
        .unwrap()
        .unwrap();
    tx.rollback().await.unwrap();
    assert!(!after.recent_primary);
    assert!(!after.recent_strong);
}

#[tokio::test]
async fn binding_an_issued_session_never_overwrites_an_existing_binding() {
    let db = isolated_database().await;
    let (_, token) = session(&db).await;
    let guard = guard(&db);
    guard.bind_issued(&token, &[1; 32]).await.unwrap();
    guard.bind_issued(&token, &[2; 32]).await.unwrap();

    assert_eq!(
        guard.observe(&token, &[1; 32]).await.unwrap(),
        Observation::Unchanged
    );
}

#[tokio::test]
async fn unknown_or_revoked_sessions_are_ignored() {
    let db = isolated_database().await;
    let (id, token) = session(&db).await;
    let guard = guard(&db);
    assert_eq!(
        guard
            .observe(&crate::oauth::store::random_secret(), &[1; 32])
            .await
            .unwrap(),
        Observation::Unknown
    );
    sqlx::query("UPDATE identity_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(id)
        .execute(&db)
        .await
        .unwrap();
    assert_eq!(
        guard.observe(&token, &[1; 32]).await.unwrap(),
        Observation::Unknown
    );
}
