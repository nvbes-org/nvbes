use super::{request_recovery, reset_password};
use crate::{
    auth,
    recovery_test_fixture::{INITIAL, fixture},
};

#[tokio::test]
async fn notification_failure_rolls_back_reset_and_outbox() {
    let (db, principal, email) = fixture().await;
    auth::authenticate(&db, &email, INITIAL).await.unwrap();
    let recovery = request_recovery(&db, &email).await.unwrap();
    sqlx::raw_sql("CREATE FUNCTION fail_password_notification() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected notification storage failure'; END $$; CREATE TRIGGER fail_password_notification BEFORE INSERT ON identity_security_notifications FOR EACH ROW EXECUTE FUNCTION fail_password_notification();")
        .execute(&db).await.unwrap();
    assert!(
        reset_password(&db, &recovery.token, "New-notified-password!")
            .await
            .is_err()
    );
    let counts:(i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM identity_outbox WHERE event_type='identity.password_recovered'),(SELECT count(*) FROM identity_audit_events WHERE event_type='identity.password_recovered'),(SELECT count(*) FROM identity_sessions WHERE revoked_at IS NULL),(SELECT count(*) FROM identity_recovery_challenges WHERE consumed_at IS NULL)")
        .fetch_one(&db).await.unwrap();
    assert_eq!(counts, (0, 0, 1, 1));
    auth::authenticate(&db, &email, INITIAL).await.unwrap();
    sqlx::query("DROP TRIGGER fail_password_notification ON identity_security_notifications")
        .execute(&db)
        .await
        .unwrap();
    reset_password(&db, &recovery.token, "New-notified-password!")
        .await
        .unwrap();
    assert!(
        reset_password(&db, &recovery.token, "Another-notified-password!")
            .await
            .is_err()
    );
    let notices: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_security_notifications WHERE principal_id=$1",
    )
    .bind(principal)
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(notices, 1);
    db.close().await;
}

#[tokio::test]
async fn password_notification_snapshots_only_verified_recipients_without_credentials() {
    let (db, principal, email) = fixture().await;
    for (recipient, verified) in [
        ("other@example.invalid", true),
        ("unverified@example.invalid", false),
    ] {
        sqlx::query("INSERT INTO identity_login_identifiers(id,principal_id,kind,normalized_value,verified_at) VALUES($1,$2,'email',$3,CASE WHEN $4 THEN clock_timestamp() ELSE NULL END)")
            .bind(uuid::Uuid::new_v4()).bind(principal).bind(recipient).bind(verified).execute(&db).await.unwrap();
    }
    let recovery = request_recovery(&db, &email).await.unwrap();
    reset_password(&db, &recovery.token, "New-notified-password!")
        .await
        .unwrap();
    let rows:Vec<serde_json::Value>=sqlx::query_scalar("SELECT command FROM identity_security_notifications WHERE principal_id=$1 ORDER BY created_at")
        .bind(principal).fetch_all(&db).await.unwrap();
    assert_eq!(rows.len(), 2);
    for row in rows {
        let serialized = row.to_string();
        assert!(!serialized.contains(&recovery.token));
        assert!(!serialized.contains("New-notified-password!"));
        assert!(!serialized.contains("unverified@example.invalid"));
        let command: nvbes_email::EmailCommand = serde_json::from_value(row).unwrap();
        assert!(matches!(
            command.template,
            nvbes_email::EmailTemplate::AccountSecurityV1 {
                event: nvbes_email::AccountSecurityEvent::PasswordRecovered,
                ..
            }
        ));
        assert_eq!(
            command.category,
            nvbes_email::EmailCategory::AccountSecurity
        );
    }
    db.close().await;
}
