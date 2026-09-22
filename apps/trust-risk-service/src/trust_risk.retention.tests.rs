use crate::config::RetentionConfig;

#[test]
fn retention_classes_remain_independent() {
    let policy = RetentionConfig {
        signals_days: 30,
        evaluations_days: 400,
        labels_days: 500,
        reviews_days: 600,
        audit_days: 730,
    };
    assert!(policy.signals_days < policy.evaluations_days);
    assert!(policy.evaluations_days < policy.audit_days);
}

#[cfg(feature = "database-tests")]
mod database {
    use sha2::{Digest, Sha256};
    use uuid::Uuid;

    use crate::config::RetentionConfig;

    use super::super::{ErasureError, apply, erase_subject};

    #[sqlx::test(migrations = "./migrations")]
    async fn apply_deletes_expired_rows_and_preserves_legal_hold(pool: sqlx::PgPool) {
        let signal_id = Uuid::new_v4();
        let held_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO trust_risk_signals (
                id, schema_version, producer, signal_kind, occurred_at, partition_key,
                scope, fingerprint, payload, projected_at, retention_deadline, legal_hold
            ) VALUES
              ($1, 1, 'billing-checkout-fixture', 'network.reputation', clock_timestamp(),
               'regional:eu-west:network:expired', 1, $3, $4, clock_timestamp(),
               clock_timestamp() - INTERVAL '1 day', FALSE),
              ($2, 1, 'billing-checkout-fixture', 'network.reputation', clock_timestamp(),
               'regional:eu-west:network:held', 1, $5, $6, clock_timestamp(),
               clock_timestamp() - INTERVAL '1 day', TRUE)
            "#,
        )
        .bind(signal_id)
        .bind(held_id)
        .bind(vec![1_u8; 32])
        .bind(vec![2_u8; 16])
        .bind(vec![3_u8; 32])
        .bind(vec![4_u8; 16])
        .execute(&pool)
        .await
        .expect("seed signals");

        sqlx::query(
            r#"
            INSERT INTO trust_risk_feature_state (
                subject_kind, namespace, opaque_id, feature_version, features,
                event_watermark, expires_at
            ) VALUES (1, 'nvbes.identity', 'principal:expired-feature', 'features-v1',
                      '{}'::jsonb, clock_timestamp(), clock_timestamp() - INTERVAL '1 hour')
            "#,
        )
        .execute(&pool)
        .await
        .expect("seed feature");

        let result = apply(&pool).await.expect("retention apply");
        assert_eq!(result.signals, 1);
        assert_eq!(result.features, 1);

        let remaining: i64 =
            sqlx::query_scalar("SELECT count(*) FROM trust_risk_signals WHERE id = $1")
                .bind(held_id)
                .fetch_one(&pool)
                .await
                .expect("held signal");
        assert_eq!(remaining, 1);
        let deleted: i64 =
            sqlx::query_scalar("SELECT count(*) FROM trust_risk_signals WHERE id = $1")
                .bind(signal_id)
                .fetch_one(&pool)
                .await
                .expect("expired signal");
        assert_eq!(deleted, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn erase_subject_removes_rows_and_rejects_invalid_input(pool: sqlx::PgPool) {
        let retention = RetentionConfig {
            signals_days: 30,
            evaluations_days: 400,
            labels_days: 400,
            reviews_days: 400,
            audit_days: 730,
        };
        assert!(matches!(
            erase_subject(
                &pool,
                0,
                "nvbes.identity",
                "principal:x",
                "ops",
                "erase",
                retention
            )
            .await,
            Err(ErasureError::InvalidInput)
        ));

        let signal_id = Uuid::new_v4();
        let fingerprint = Sha256::digest(b"erase-fixture").to_vec();
        sqlx::query(
            r#"
            INSERT INTO trust_risk_signals (
                id, schema_version, producer, signal_kind, occurred_at, partition_key,
                scope, fingerprint, payload, retention_deadline
            ) VALUES ($1, 1, 'billing-checkout-fixture', 'network.reputation', clock_timestamp(),
                      'regional:eu-west:principal:erase', 1, $2, $3,
                      clock_timestamp() + INTERVAL '30 days')
            "#,
        )
        .bind(signal_id)
        .bind(&fingerprint)
        .bind(vec![9_u8; 8])
        .execute(&pool)
        .await
        .expect("seed signal");
        sqlx::query(
            r#"
            INSERT INTO trust_risk_signal_subjects (
                signal_id, ordinal, kind, namespace, opaque_id, scope
            ) VALUES ($1, 0, 1, 'nvbes.identity', 'principal:erase-me-ok', 1)
            "#,
        )
        .bind(signal_id)
        .execute(&pool)
        .await
        .expect("seed subject");

        let request_id = erase_subject(
            &pool,
            1,
            "nvbes.identity",
            "principal:erase-me-ok",
            "operator-ada",
            "subject requested erasure",
            retention,
        )
        .await
        .expect("erase");
        assert!(!request_id.is_nil());

        let remaining: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM trust_risk_signal_subjects WHERE opaque_id = 'principal:erase-me-ok'",
        )
        .fetch_one(&pool)
        .await
        .expect("subjects gone");
        assert_eq!(remaining, 0);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn erase_subject_rejects_legal_hold(pool: sqlx::PgPool) {
        let retention = RetentionConfig {
            signals_days: 30,
            evaluations_days: 400,
            labels_days: 400,
            reviews_days: 400,
            audit_days: 730,
        };
        let signal_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO trust_risk_signals (
                id, schema_version, producer, signal_kind, occurred_at, partition_key,
                scope, fingerprint, payload, retention_deadline, legal_hold
            ) VALUES ($1, 1, 'billing-checkout-fixture', 'network.reputation', clock_timestamp(),
                      'regional:eu-west:principal:hold', 1, $2, $3,
                      clock_timestamp() + INTERVAL '30 days', TRUE)
            "#,
        )
        .bind(signal_id)
        .bind(vec![7_u8; 32])
        .bind(vec![8_u8; 8])
        .execute(&pool)
        .await
        .expect("seed held");
        sqlx::query(
            r#"
            INSERT INTO trust_risk_signal_subjects (
                signal_id, ordinal, kind, namespace, opaque_id, scope
            ) VALUES ($1, 0, 1, 'nvbes.identity', 'principal:legal-hold', 1)
            "#,
        )
        .bind(signal_id)
        .execute(&pool)
        .await
        .expect("seed subject");

        assert!(matches!(
            erase_subject(
                &pool,
                1,
                "nvbes.identity",
                "principal:legal-hold",
                "operator-ada",
                "attempt erasure under hold",
                retention,
            )
            .await,
            Err(ErasureError::LegalHold)
        ));
    }
}
