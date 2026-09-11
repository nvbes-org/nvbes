use super::*;
use crate::test_fixtures::isolated_database;
use chrono::{Duration, Utc};
use std::sync::{Arc, Mutex};

async fn queued(db: &PgPool, addresses: &[(&str, bool)]) -> (Uuid, Uuid) {
    let principal = Uuid::new_v4();
    let event = Uuid::new_v4();
    let mut tx = db.begin().await.unwrap();
    sqlx::query("INSERT INTO identity_principals(id,kind,status) VALUES($1,'human','active')")
        .bind(principal)
        .execute(&mut *tx)
        .await
        .unwrap();
    for (address, verified) in addresses {
        sqlx::query("INSERT INTO identity_login_identifiers(id,principal_id,kind,normalized_value,verified_at) VALUES($1,$2,'email',$3,CASE WHEN $4 THEN clock_timestamp() ELSE NULL END)")
            .bind(Uuid::new_v4()).bind(principal).bind(address).bind(verified).execute(&mut *tx).await.unwrap();
    }
    let occurred = sqlx::query_scalar("INSERT INTO identity_outbox(id,event_type,aggregate_id,payload) VALUES($1,'identity.mfa_recovered',$2,'{}') RETURNING occurred_at")
        .bind(event).bind(principal).fetch_one(&mut *tx).await.unwrap();
    crate::notification_queue::enqueue(
        &mut tx,
        event,
        principal,
        "identity.mfa_recovered",
        occurred,
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();
    (principal, event)
}
fn receipt(command: &EmailCommand) -> EmailReceipt {
    EmailReceipt {
        message_id: Uuid::new_v4().to_string(),
        accepted_at: Utc::now(),
        deliver_before: command.deliver_before,
        duplicate: false,
    }
}

#[tokio::test]
async fn concurrent_dispatchers_send_each_verified_snapshot_once() {
    let db = isolated_database().await;
    queued(
        &db,
        &[
            ("one@example.invalid", true),
            ("two@example.invalid", true),
            ("unverified@example.invalid", false),
        ],
    )
    .await;
    let sent = Arc::new(Mutex::new(Vec::new()));
    let send = |command: EmailCommand| {
        let sent = sent.clone();
        async move {
            sent.lock().unwrap().push(command.clone());
            tokio::task::yield_now().await;
            Ok(receipt(&command))
        }
    };
    let (a, b) = tokio::join!(run_with(&db, &send), run_with(&db, &send));
    assert_eq!(a.unwrap().accepted + b.unwrap().accepted, 2);
    assert_eq!(run_with(&db, &send).await.unwrap().claimed, 0);
    let commands = sent.lock().unwrap();
    assert_eq!(commands.len(), 2);
    assert_ne!(commands[0].idempotency_key, commands[1].idempotency_key);
    assert!(
        !commands
            .iter()
            .any(|c| c.recipient.email.starts_with("unverified"))
    );
    let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_security_notifications WHERE command IS NOT NULL OR receipt_id IS NULL OR state<>'accepted'").fetch_one(&db).await.unwrap();
    assert_eq!(retained, 0);
}

#[tokio::test]
async fn unavailable_delivery_retries_identically_after_identifier_changes_and_process_loss() {
    let db = isolated_database().await;
    let (principal, _) = queued(&db, &[("original@example.invalid", true)]).await;
    let sent = Arc::new(Mutex::new(Vec::new()));
    let result = run_with(&db, |command| {
        let sent = sent.clone();
        async move {
            sent.lock().unwrap().push(command);
            Err(EmailClientError::Unavailable)
        }
    })
    .await
    .unwrap();
    assert_eq!(result.deferred, 1);
    assert_eq!(
        run_with(&db, |_| async { panic!("backoff must prevent send") })
            .await
            .unwrap()
            .claimed,
        0
    );
    sqlx::query("UPDATE identity_login_identifiers SET normalized_value='changed@example.invalid' WHERE principal_id=$1").bind(principal).execute(&db).await.unwrap();
    // Simulate a process disappearing after its durable claim, before recording a receipt.
    sqlx::query("UPDATE identity_security_notifications SET state='sending',lease_token=$1,lease_expires_at=clock_timestamp()-interval '1 second',attempts=2").bind(Uuid::new_v4()).execute(&db).await.unwrap();
    let expected = sent.lock().unwrap()[0].clone();
    let result = run_with(&db, |command| {
        assert_eq!(command, expected);
        async move {
            let mut value = receipt(&command);
            value.duplicate = true;
            Ok(value)
        }
    })
    .await
    .unwrap();
    assert_eq!(result.accepted, 1);
    let attempts: i16 = sqlx::query_scalar("SELECT attempts FROM identity_security_notifications")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(attempts, 3);
}

#[tokio::test]
async fn missing_recipient_expiration_and_exhaustion_never_claim_success() {
    let db = isolated_database().await;
    queued(&db, &[("pending@example.invalid", false)]).await;
    queued(&db, &[("expired@example.invalid", true)]).await;
    sqlx::query("UPDATE identity_security_notifications SET deliver_before=clock_timestamp()-interval '1 second' WHERE state='pending'").execute(&db).await.unwrap();
    queued(&db, &[("exhausted@example.invalid", true)]).await;
    sqlx::query("UPDATE identity_security_notifications SET state='sending',attempts=8,lease_token=$1,lease_expires_at=clock_timestamp()-interval '1 second' WHERE state='pending'").bind(Uuid::new_v4()).execute(&db).await.unwrap();
    assert_eq!(
        run_with(&db, |_| async { panic!("terminal rows must not be sent") })
            .await
            .unwrap()
            .claimed,
        0
    );
    let states: Vec<String> =
        sqlx::query_scalar("SELECT state FROM identity_security_notifications ORDER BY state")
            .fetch_all(&db)
            .await
            .unwrap();
    assert_eq!(states, vec!["expired", "failed", "no_recipient"]);
}

#[tokio::test]
async fn stale_receipt_cannot_overwrite_a_new_lease_and_invalid_receipt_fails() {
    let db = isolated_database().await;
    queued(&db, &[("fenced@example.invalid", true)]).await;
    let result = run_with(&db, |command| {
        let db = db.clone();
        async move {
            sqlx::query("UPDATE identity_security_notifications SET lease_token=$1")
                .bind(Uuid::new_v4())
                .execute(&db)
                .await
                .unwrap();
            Ok(receipt(&command))
        }
    })
    .await
    .unwrap();
    assert_eq!(result.accepted, 0);
    sqlx::query("UPDATE identity_security_notifications SET lease_expires_at=clock_timestamp()-interval '1 second'").execute(&db).await.unwrap();
    let result = run_with(&db, |command| async move {
        let mut value = receipt(&command);
        value.deliver_before += Duration::hours(1);
        Ok(value)
    })
    .await
    .unwrap();
    assert_eq!(result.failed, 1);
}

#[tokio::test]
async fn real_mfa_generation_enqueues_atomically_and_retention_clears_only_settled_metadata() {
    let db = isolated_database().await;
    let (session_id, token) = crate::test_fixtures::session(&db).await;
    let principal: Uuid =
        sqlx::query_scalar("SELECT principal_id FROM identity_sessions WHERE id=$1")
            .bind(session_id)
            .fetch_one(&db)
            .await
            .unwrap();
    sqlx::query("INSERT INTO identity_login_identifiers(id,principal_id,kind,normalized_value,verified_at) VALUES($1,$2,'email','owner@example.invalid',clock_timestamp())")
        .bind(Uuid::new_v4()).bind(principal).execute(&db).await.unwrap();
    let crypto = crate::mfa_crypto::MfaCrypto::with_rotation(1, [8; 32], None).unwrap();
    let pending = crate::totp::start(&db, &crypto, &token).await.unwrap();
    let code = nvbes_core::mfa::generate_totp_code(
        &pending.secret_base32,
        nvbes_core::mfa::current_counter(Utc::now()),
    );
    crate::totp::confirm(&db, &crypto, &token, pending.factor_id, &code)
        .await
        .unwrap();
    let initial = crate::mfa_recovery::generate(&db, &token).await.unwrap();
    sqlx::query("ALTER TABLE identity_security_notifications ADD CONSTRAINT injected_queue_failure CHECK (state<>'pending') NOT VALID").execute(&db).await.unwrap();
    assert!(crate::mfa_recovery::generate(&db, &token).await.is_err());
    sqlx::query(
        "ALTER TABLE identity_security_notifications DROP CONSTRAINT injected_queue_failure",
    )
    .execute(&db)
    .await
    .unwrap();
    // Failed regeneration rolled back deletion of the previous codes.
    crate::mfa_recovery::redeem(&db, &token, &initial.codes[0])
        .await
        .unwrap();
    assert_eq!(
        run_with(&db, |command| async move { Ok(receipt(&command)) })
            .await
            .unwrap()
            .accepted,
        2
    );
    let accepted: i64=sqlx::query_scalar("SELECT count(*) FROM identity_security_notifications WHERE email_accepted_at IS NOT NULL AND command IS NULL").fetch_one(&db).await.unwrap();
    assert_eq!(accepted, 2);
    sqlx::query("UPDATE identity_security_notifications SET settled_at=clock_timestamp()-interval '31 days'").execute(&db).await.unwrap();
    run_with(&db, |_| async { panic!("settled records must not replay") })
        .await
        .unwrap();
    let remaining: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_security_notifications")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(remaining, 0);
    let audit: i64=sqlx::query_scalar("SELECT count(*) FROM identity_audit_events WHERE principal_id=$1 AND event_type='identity.mfa_recovery_codes_generated'").bind(principal).fetch_one(&db).await.unwrap();
    assert_eq!(audit, 1);
}
