use super::*;
use crate::recovery_test_fixture::fixture;
use std::sync::{Arc, Mutex};

const BASE: &str = "https://identity.example/recover";
fn crypto() -> MfaCrypto {
    MfaCrypto::with_rotation(1, [31; 32], None).unwrap()
}
fn receipt(command: &EmailCommand) -> EmailReceipt {
    EmailReceipt {
        message_id: Uuid::new_v4().to_string(),
        accepted_at: chrono::Utc::now(),
        deliver_before: command.deliver_before,
        duplicate: false,
    }
}
async fn retained(db: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM identity_recovery_deliveries WHERE ciphertext IS NOT NULL OR nonce IS NOT NULL OR key_version IS NOT NULL")
        .fetch_one(db).await.unwrap()
}

#[tokio::test]
async fn enqueue_commits_only_encrypted_commands_and_rolls_back_invalid_delivery() {
    let (db, principal, email) = fixture().await;
    let crypto = crypto();
    assert!(
        enqueue(&db, &crypto, "http://invalid.example/recover", &email)
            .await
            .is_err()
    );
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM identity_recovery_challenges),(SELECT count(*) FROM identity_audit_events WHERE event_type='identity.recovery_requested')")
        .fetch_one(&db).await.unwrap();
    assert_eq!(counts, (0, 0));
    sqlx::raw_sql("CREATE FUNCTION fail_enqueue() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected enqueue failure'; END $$; CREATE TRIGGER fail_enqueue BEFORE INSERT ON identity_recovery_deliveries FOR EACH ROW EXECUTE FUNCTION fail_enqueue();")
        .execute(&db).await.unwrap();
    assert!(enqueue(&db, &crypto, BASE, &email).await.is_err());
    let after_failure: i64 =
        sqlx::query_scalar("SELECT count(*) FROM identity_recovery_challenges")
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(after_failure, 0);
    sqlx::query("DROP TRIGGER fail_enqueue ON identity_recovery_deliveries")
        .execute(&db)
        .await
        .unwrap();
    let challenge = enqueue(&db, &crypto, BASE, &email).await.unwrap();
    let row: Delivery =
        sqlx::query_as("SELECT * FROM identity_recovery_deliveries WHERE challenge_id=$1")
            .bind(challenge)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(!String::from_utf8_lossy(&row.ciphertext).contains(&email));
    let command = prepare(&db, &crypto, &row).await.unwrap();
    assert_eq!(command.recipient.email, email);
    let nvbes_email::EmailTemplate::PasswordResetV1 { reset_url, .. } = command.template else {
        panic!("wrong template")
    };
    let token = reqwest::Url::parse(&reset_url)
        .unwrap()
        .query_pairs()
        .find(|(k, _)| k == "token")
        .unwrap()
        .1
        .to_string();
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT token_hash FROM identity_recovery_challenges WHERE id=$1")
            .bind(challenge)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(stored, crate::auth::hash_token(&token));
    assert!(
        sqlx::query("UPDATE identity_recovery_deliveries SET key_version=NULL")
            .execute(&db)
            .await
            .is_err()
    );
    crate::recovery::reset_password(&db, &token, "Delivered-reset-new-password!")
        .await
        .unwrap();
    assert_eq!(
        retained(&db).await,
        0,
        "reset clears delivery secrets atomically"
    );
    assert_eq!(
        run_with(&db, &crypto, |_| async {
            panic!("consumed reset link must not be sent")
        })
        .await
        .unwrap()
        .claimed,
        0
    );
    assert_eq!(retained(&db).await, 0);
    let status: String =
        sqlx::query_scalar("SELECT state FROM identity_recovery_deliveries WHERE principal_id=$1")
            .bind(principal)
            .fetch_one(&db)
            .await
            .unwrap();
    assert_eq!(status, "cancelled");
    db.close().await;
}

#[tokio::test]
async fn retries_keep_identical_command_across_rotation_and_concurrent_workers() {
    let (db, _, email) = fixture().await;
    let crypto = crypto();
    enqueue(&db, &crypto, BASE, &email).await.unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let result = run_with(&db, &crypto, |command| {
        calls.lock().unwrap().push(command);
        async { Err(EmailClientError::Unavailable) }
    })
    .await
    .unwrap();
    assert_eq!(result.deferred, 1);
    assert_eq!(retained(&db).await, 1);
    assert_eq!(
        run_with(&db, &crypto, |_| async { panic!("backoff ignored") })
            .await
            .unwrap()
            .claimed,
        0
    );
    sqlx::query("UPDATE identity_recovery_deliveries SET available_at=clock_timestamp()")
        .execute(&db)
        .await
        .unwrap();
    let rotating = MfaCrypto::with_rotation(2, [32; 32], Some((1, [31; 32]))).unwrap();
    let expected = calls.lock().unwrap()[0].clone();
    let send = |command: EmailCommand| {
        assert_eq!(command, expected);
        calls.lock().unwrap().push(command.clone());
        async move {
            tokio::task::yield_now().await;
            Ok(receipt(&command))
        }
    };
    let (a, b) = tokio::join!(
        run_with(&db, &rotating, &send),
        run_with(&db, &rotating, &send)
    );
    assert_eq!(a.unwrap().accepted + b.unwrap().accepted, 1);
    assert_eq!(calls.lock().unwrap().len(), 2);
    assert_eq!(retained(&db).await, 0);
    db.close().await;
}

#[tokio::test]
async fn accepted_email_can_be_reconciled_after_losing_the_database_acknowledgement() {
    let (db, _, email) = fixture().await;
    let crypto = crypto();
    enqueue(&db, &crypto, BASE, &email).await.unwrap();
    sqlx::raw_sql("CREATE FUNCTION fail_delivery_ack() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.state='accepted' THEN RAISE EXCEPTION 'injected acknowledgement failure'; END IF; RETURN NEW; END $$; CREATE TRIGGER fail_delivery_ack BEFORE UPDATE ON identity_recovery_deliveries FOR EACH ROW EXECUTE FUNCTION fail_delivery_ack();")
        .execute(&db).await.unwrap();
    let accepted = Arc::new(Mutex::new(None));
    assert!(
        run_with(&db, &crypto, |command| {
            let received = receipt(&command);
            *accepted.lock().unwrap() = Some((command, received.clone()));
            async move { Ok(received) }
        })
        .await
        .is_err()
    );
    assert_eq!(retained(&db).await, 1);
    sqlx::raw_sql("DROP TRIGGER fail_delivery_ack ON identity_recovery_deliveries; UPDATE identity_recovery_deliveries SET lease_expires_at=clock_timestamp()-interval '1 second';")
        .execute(&db).await.unwrap();
    let (expected, mut received) = accepted.lock().unwrap().clone().unwrap();
    received.duplicate = true;
    let result = run_with(&db, &crypto, |command| {
        assert_eq!(command, expected);
        let received = received.clone();
        async move { Ok(received) }
    })
    .await
    .unwrap();
    assert_eq!(result.accepted, 1);
    assert_eq!(retained(&db).await, 0);
    db.close().await;
}

#[tokio::test]
async fn obsolete_unreadable_and_unverified_deliveries_are_never_sent() {
    for mutation in [
        "UPDATE identity_recovery_deliveries SET deliver_before=clock_timestamp()-interval '1 second'",
        "UPDATE identity_recovery_deliveries SET key_version=99",
        "UPDATE identity_login_identifiers SET verified_at=NULL",
        "UPDATE identity_principals SET status='suspended'",
        "UPDATE identity_recovery_deliveries SET attempts=8,state='sending',lease_token=gen_random_uuid(),lease_expires_at=clock_timestamp()-interval '1 second'",
    ] {
        let (db, _, email) = fixture().await;
        let crypto = crypto();
        enqueue(&db, &crypto, BASE, &email).await.unwrap();
        sqlx::query(mutation).execute(&db).await.unwrap();
        run_with(&db, &crypto, |_| async {
            panic!("invalid delivery must not reach Email")
        })
        .await
        .unwrap();
        assert_eq!(retained(&db).await, 0);
        db.close().await;
    }
}

#[tokio::test]
async fn batches_are_bounded_and_invalid_receipts_are_not_successes() {
    let (db, _, email) = fixture().await;
    let crypto = crypto();
    for _ in 0..17 {
        enqueue(&db, &crypto, BASE, &email).await.unwrap();
    }
    let result = run_with(&db, &crypto, |command| async move { Ok(receipt(&command)) })
        .await
        .unwrap();
    assert_eq!(result.claimed, 16);
    assert_eq!(result.accepted, 16);
    assert_eq!(retained(&db).await, 1);
    let result = run_with(&db, &crypto, |command| async move {
        let mut invalid = receipt(&command);
        invalid.deliver_before += chrono::Duration::seconds(1);
        Ok(invalid)
    })
    .await
    .unwrap();
    assert_eq!(result.accepted, 0);
    assert_eq!(result.failed, 1);
    assert_eq!(retained(&db).await, 0);
    db.close().await;
}
