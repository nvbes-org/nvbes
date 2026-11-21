use crate::test_fixtures::{isolated_database, session};

#[tokio::test]
async fn future_primary_does_not_consume_a_valid_recovery_code() {
    let db = isolated_database().await;
    let (id, token) = session(&db).await;
    let crypto = crate::mfa_crypto::MfaCrypto::with_rotation(1, [8; 32], None).unwrap();
    let pending = crate::totp::start(&db, &crypto, &token).await.unwrap();
    let code = nvbes_core::mfa::generate_totp_code(
        &pending.secret_base32,
        nvbes_core::mfa::current_counter(chrono::Utc::now()),
    );
    crate::totp::confirm(&db, &crypto, &token, pending.factor_id, &code)
        .await
        .unwrap();
    let codes = crate::mfa_recovery::generate(&db, &token).await.unwrap();
    sqlx::query("UPDATE identity_sessions SET authenticated_at=clock_timestamp()+interval '1 hour',step_up_method=NULL,step_up_at=NULL,step_up_expires_at=NULL WHERE id=$1")
        .bind(id).execute(&db).await.unwrap();
    assert!(
        crate::mfa_recovery::redeem(&db, &token, &codes.codes[0])
            .await
            .is_err()
    );
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM identity_mfa_recovery_codes WHERE consumed_at IS NULL",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(remaining, 10);
    let live: bool =
        sqlx::query_scalar("SELECT revoked_at IS NULL FROM identity_sessions WHERE id=$1")
            .bind(id)
            .fetch_one(&db)
            .await
            .unwrap();
    assert!(live);
    sqlx::query("UPDATE identity_sessions SET authenticated_at=clock_timestamp()-interval '1 minute' WHERE id=$1")
        .bind(id).execute(&db).await.unwrap();
    assert!(
        crate::mfa_recovery::redeem(&db, &token, &codes.codes[0])
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn future_and_inconsistent_evidence_never_authorizes_factor_changes() {
    let db = isolated_database().await;
    let (id, token) = session(&db).await;
    let mutations = [
        "authenticated_at=clock_timestamp()+interval '1 hour'",
        "authenticated_at=clock_timestamp()-interval '1 minute',step_up_method='totp',step_up_at=clock_timestamp()+interval '1 minute',step_up_expires_at=clock_timestamp()+interval '10 minutes'",
        "authenticated_at=clock_timestamp()-interval '1 minute',step_up_method='totp',step_up_at=clock_timestamp()-interval '2 minutes',step_up_expires_at=clock_timestamp()+interval '10 minutes'",
    ];
    for mutation in mutations {
        sqlx::query(&format!(
            "UPDATE identity_sessions SET {mutation} WHERE id=$1"
        ))
        .bind(id)
        .execute(&db)
        .await
        .unwrap();
        let mut tx = db.begin().await.unwrap();
        assert!(super::load(&mut tx, &token).await.unwrap().is_none());
        assert!(
            crate::enrollment_policy::owner(&mut tx, &token)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            crate::factor_management::owner(&mut tx, &token, true)
                .await
                .unwrap()
                .is_none()
        );
        tx.rollback().await.unwrap();
        let crypto = crate::mfa_crypto::MfaCrypto::with_rotation(1, [8; 32], None).unwrap();
        assert!(crate::totp::start(&db, &crypto, &token).await.is_err());
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_auth_factors")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[tokio::test]
async fn recent_primary_and_strong_proofs_have_distinct_permissions() {
    let db = isolated_database().await;
    let (id, token) = session(&db).await;
    let mut tx = db.begin().await.unwrap();
    assert!(
        crate::enrollment_policy::owner(&mut tx, &token)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        crate::factor_management::owner(&mut tx, &token, true)
            .await
            .unwrap()
            .is_none()
    );
    tx.rollback().await.unwrap();
    sqlx::query("UPDATE identity_sessions SET authenticated_at=clock_timestamp()-interval '20 minutes',step_up_method='totp',step_up_at=clock_timestamp()-interval '1 minute',step_up_expires_at=clock_timestamp()+interval '5 minutes' WHERE id=$1")
        .bind(id).execute(&db).await.unwrap();
    let mut tx = db.begin().await.unwrap();
    let evidence = super::load(&mut tx, &token).await.unwrap().unwrap();
    assert!(!evidence.recent_primary);
    assert!(evidence.recent_strong);
    assert!(
        crate::enrollment_policy::owner(&mut tx, &token)
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        crate::factor_management::owner(&mut tx, &token, true)
            .await
            .unwrap()
            .is_some()
    );
    tx.rollback().await.unwrap();
    sqlx::query("UPDATE identity_sessions SET step_up_at=clock_timestamp()-interval '6 minutes' WHERE id=$1")
        .bind(id).execute(&db).await.unwrap();
    let mut tx = db.begin().await.unwrap();
    assert!(
        crate::enrollment_policy::owner(&mut tx, &token)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        crate::factor_management::owner(&mut tx, &token, true)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        crate::factor_management::owner(&mut tx, &token, false)
            .await
            .unwrap()
            .is_some()
    );
}
