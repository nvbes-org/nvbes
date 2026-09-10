use super::{request_recovery, reset_password};
use crate::{auth, database};
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;
use uuid::Uuid;

const INITIAL: &str = "Initial-recovery-database-password!";
const REPLACEMENT: &str = "Replacement-recovery-database-password!";

async fn fixture() -> (PgPool, Uuid, String) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
    let admin = database::connect(&url, 1).await.unwrap();
    let schema = format!("identity_password_recovery_{}", Uuid::new_v4().simple());
    sqlx::query(&format!("CREATE SCHEMA {schema}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
    let db = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(move |connection, _| {
            let schema = schema.clone();
            Box::pin(async move {
                sqlx::query("SELECT set_config('search_path',$1,false),set_config('application_name',$1,false)")
                    .bind(schema)
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&url)
        .await
        .unwrap();
    database::migrate(&db).await.unwrap();
    let email = format!("recovery-{}@example.invalid", Uuid::new_v4());
    let principal = auth::create_synthetic_identity(&db, &email, INITIAL)
        .await
        .unwrap();
    (db, principal, email)
}

async fn live_sessions(db: &PgPool, principal: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM identity_sessions WHERE principal_id=$1 AND revoked_at IS NULL",
    )
    .bind(principal)
    .fetch_one(db)
    .await
    .unwrap()
}

async fn pending_links(db: &PgPool, principal: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM identity_recovery_challenges WHERE principal_id=$1 AND consumed_at IS NULL")
        .bind(principal).fetch_one(db).await.unwrap()
}

async fn reset_audits(db: &PgPool, principal: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM identity_audit_events WHERE principal_id=$1 AND event_type='identity.password_recovered'")
        .bind(principal).fetch_one(db).await.unwrap()
}

#[tokio::test]
async fn concurrent_recovery_links_have_one_winner_and_revoke_every_session() {
    for same_link in [true, false] {
        let (db, principal, email) = fixture().await;
        auth::authenticate(&db, &email, INITIAL).await.unwrap();
        auth::authenticate(&db, &email, INITIAL).await.unwrap();
        let first = request_recovery(&db, &email).await.unwrap();
        let sibling = request_recovery(&db, &email).await.unwrap();
        let other_email = format!("unrelated-{}@example.invalid", Uuid::new_v4());
        let other = auth::create_synthetic_identity(&db, &other_email, INITIAL)
            .await
            .unwrap();
        auth::authenticate(&db, &other_email, INITIAL)
            .await
            .unwrap();
        request_recovery(&db, &other_email).await.unwrap();
        let second = if same_link {
            &first.token
        } else {
            &sibling.token
        };
        let results = tokio::time::timeout(Duration::from_secs(15), async {
            tokio::join!(
                reset_password(&db, &first.token, REPLACEMENT),
                reset_password(&db, second, "Another-recovery-database-password!")
            )
        })
        .await
        .expect("no deadlock between recovery links");
        assert_ne!(results.0.is_ok(), results.1.is_ok());
        assert_eq!(live_sessions(&db, principal).await, 0);
        assert_eq!(pending_links(&db, principal).await, 0);
        assert_eq!(reset_audits(&db, principal).await, 1);
        assert_eq!(live_sessions(&db, other).await, 1);
        assert_eq!(pending_links(&db, other).await, 1);
        assert!(reset_password(&db, &sibling.token, INITIAL).await.is_err());
        assert!(auth::authenticate(&db, &email, INITIAL).await.is_err());
        let winner = if results.0.is_ok() {
            REPLACEMENT
        } else {
            "Another-recovery-database-password!"
        };
        auth::authenticate(&db, &email, winner).await.unwrap();
        db.close().await;
    }
}

#[tokio::test]
async fn recovery_invalidates_previously_verified_login_without_granting_mfa() {
    let (db, principal, email) = fixture().await;
    let verified = auth::verify_credentials(&db, &email, INITIAL)
        .await
        .unwrap();
    let recovery = request_recovery(&db, &email).await.unwrap();
    reset_password(&db, &recovery.token, REPLACEMENT)
        .await
        .unwrap();
    let mut tx = db.begin().await.unwrap();
    assert!(
        auth::create_verified_session(&mut tx, verified)
            .await
            .is_err()
    );
    tx.rollback().await.unwrap();
    assert_eq!(live_sessions(&db, principal).await, 0);
    let token = auth::authenticate(&db, &email, REPLACEMENT).await.unwrap();
    let primary: String =
        sqlx::query_scalar("SELECT primary_amr FROM identity_sessions WHERE token_hash=$1")
            .bind(auth::hash_token(&token))
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(primary, "pwd");
    db.close().await;
}

#[tokio::test]
async fn invalid_expired_and_unchanged_password_attempts_preserve_account_state() {
    let (db, principal, email) = fixture().await;
    auth::authenticate(&db, &email, INITIAL).await.unwrap();
    let recovery = request_recovery(&db, &email).await.unwrap();
    assert!(reset_password(&db, "wrong", REPLACEMENT).await.is_err());
    assert!(
        reset_password(&db, &auth::random_token(), REPLACEMENT)
            .await
            .is_err()
    );
    assert!(reset_password(&db, &recovery.token, "short").await.is_err());
    assert!(reset_password(&db, &recovery.token, INITIAL).await.is_err());
    sqlx::query("UPDATE identity_recovery_challenges SET created_at=clock_timestamp()-interval '16 minutes', expires_at=clock_timestamp()-interval '1 second' WHERE id=$1")
        .bind(recovery.challenge_id).execute(&db).await.unwrap();
    assert!(
        reset_password(&db, &recovery.token, REPLACEMENT)
            .await
            .is_err()
    );
    assert_eq!(pending_links(&db, principal).await, 1);
    assert_eq!(live_sessions(&db, principal).await, 1);
    assert_eq!(reset_audits(&db, principal).await, 0);
    auth::authenticate(&db, &email, INITIAL).await.unwrap();
    db.close().await;
}

#[tokio::test]
async fn recovery_rejects_inactive_accounts_and_unverified_recipients() {
    let (db, principal, email) = fixture().await;
    let recovery = request_recovery(&db, &email).await.unwrap();
    for status in ["suspended", "closed", "pending_verification"] {
        sqlx::query("UPDATE identity_principals SET status=$1 WHERE id=$2")
            .bind(status)
            .bind(principal)
            .execute(&db)
            .await
            .unwrap();
        assert!(request_recovery(&db, &email).await.is_err());
        assert!(
            reset_password(&db, &recovery.token, REPLACEMENT)
                .await
                .is_err()
        );
    }
    sqlx::query("UPDATE identity_principals SET status='active' WHERE id=$1")
        .bind(principal)
        .execute(&db)
        .await
        .unwrap();
    sqlx::query("UPDATE identity_login_identifiers SET verified_at=NULL WHERE principal_id=$1")
        .bind(principal)
        .execute(&db)
        .await
        .unwrap();
    assert!(request_recovery(&db, &email).await.is_err());
    assert_eq!(pending_links(&db, principal).await, 1);
    assert_eq!(reset_audits(&db, principal).await, 0);
    db.close().await;
}

#[tokio::test]
async fn recovery_rechecks_expiry_and_status_after_waiting_for_principal_lock() {
    for suspend in [false, true] {
        let (db, principal, email) = fixture().await;
        let recovery = request_recovery(&db, &email).await.unwrap();
        let mut lock = db.begin().await.unwrap();
        crate::session_locks::principal(&mut lock, principal)
            .await
            .unwrap();
        let worker_db = db.clone();
        let worker =
            tokio::spawn(
                async move { reset_password(&worker_db, &recovery.token, REPLACEMENT).await },
            );
        // Synchronize on the database lock wait, not an assumed hashing duration.
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE application_name=current_schema() AND wait_event_type='Lock' AND query LIKE 'SELECT id FROM identity_principals%')")
                    .fetch_one(&db).await.unwrap();
                if waiting { break; }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        }).await.expect("reset reached the principal lock after password hashing");
        if suspend {
            sqlx::query("UPDATE identity_principals SET status='suspended' WHERE id=$1")
                .bind(principal)
                .execute(&mut *lock)
                .await
                .unwrap();
        } else {
            sqlx::query("UPDATE identity_recovery_challenges SET created_at=clock_timestamp()-interval '16 minutes',expires_at=clock_timestamp()-interval '1 second' WHERE principal_id=$1")
                .bind(principal).execute(&mut *lock).await.unwrap();
        }
        lock.commit().await.unwrap();
        assert!(worker.await.unwrap().is_err());
        assert_eq!(pending_links(&db, principal).await, 1);
        assert_eq!(reset_audits(&db, principal).await, 0);
        db.close().await;
    }
}

#[tokio::test]
async fn failed_recovery_audit_rolls_back_password_links_and_revocations() {
    let (db, principal, email) = fixture().await;
    auth::authenticate(&db, &email, INITIAL).await.unwrap();
    let first = request_recovery(&db, &email).await.unwrap();
    request_recovery(&db, &email).await.unwrap();
    sqlx::raw_sql("CREATE FUNCTION deny_recovery_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.event_type='identity.password_recovered' THEN RAISE EXCEPTION 'injected recovery audit failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER deny_recovery_audit BEFORE INSERT ON identity_audit_events FOR EACH ROW EXECUTE FUNCTION deny_recovery_audit();")
        .execute(&db).await.unwrap();
    assert!(
        reset_password(&db, &first.token, REPLACEMENT)
            .await
            .is_err()
    );
    assert_eq!(pending_links(&db, principal).await, 2);
    assert_eq!(live_sessions(&db, principal).await, 1);
    assert_eq!(reset_audits(&db, principal).await, 0);
    auth::authenticate(&db, &email, INITIAL).await.unwrap();
    sqlx::query("DROP TRIGGER deny_recovery_audit ON identity_audit_events")
        .execute(&db)
        .await
        .unwrap();
    reset_password(&db, &first.token, REPLACEMENT)
        .await
        .unwrap();
    assert_eq!(pending_links(&db, principal).await, 0);
    assert_eq!(live_sessions(&db, principal).await, 0);
    db.close().await;
}
