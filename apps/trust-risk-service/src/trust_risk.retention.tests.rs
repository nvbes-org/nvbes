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

#[test]
fn erasure_input_accepts_exact_length_boundaries() {
    assert!(super::erasure_input_is_valid(
        1, "abc", "12345678", "abc", "why"
    ));
    assert!(!super::erasure_input_is_valid(
        0, "abc", "12345678", "abc", "why"
    ));
    assert!(!super::erasure_input_is_valid(
        1, "ab", "12345678", "abc", "why"
    ));
    assert!(!super::erasure_input_is_valid(
        1, "abc", "1234567", "abc", "why"
    ));
    assert!(!super::erasure_input_is_valid(
        1, "abc", "12345678", "ab", "why"
    ));
    assert!(!super::erasure_input_is_valid(
        1, "abc", "12345678", "abc", "  "
    ));
}

#[tokio::test]
async fn retention_worker_exits_when_shutdown_is_signaled() {
    use crate::grpc_test_support;

    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/nvbes_trust_risk_unused")
        .expect("lazy pool");
    let state = grpc_test_support::state(pool);
    let (tx, rx) = tokio::sync::watch::channel(false);
    let worker = tokio::spawn(super::run(state, rx));
    tx.send(true).expect("shutdown");
    tokio::time::timeout(std::time::Duration::from_secs(2), worker)
        .await
        .expect("retention worker should exit on shutdown")
        .expect("join");
}

#[tokio::test]
async fn erase_subject_rejects_each_invalid_input_arm_without_database() {
    use super::{ErasureError, erase_subject};

    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/nvbes_trust_risk_unused")
        .expect("lazy pool");
    let retention = RetentionConfig {
        signals_days: 30,
        evaluations_days: 400,
        labels_days: 400,
        reviews_days: 400,
        audit_days: 730,
    };
    let cases = [
        (
            0_i16,
            "nvbes.identity",
            "principal:enough",
            "operator-ada",
            "valid reason",
        ),
        (1, "ab", "principal:enough", "operator-ada", "valid reason"),
        (1, "nvbes.identity", "short", "operator-ada", "valid reason"),
        (
            1,
            "nvbes.identity",
            "principal:enough",
            "ab",
            "valid reason",
        ),
        (
            1,
            "nvbes.identity",
            "principal:enough",
            "operator-ada",
            "  ",
        ),
    ];
    for (kind, namespace, opaque_id, actor, reason) in cases {
        assert!(
            matches!(
                erase_subject(&pool, kind, namespace, opaque_id, actor, reason, retention).await,
                Err(ErasureError::InvalidInput)
            ),
            "expected InvalidInput for kind={kind} ns={namespace}"
        );
    }
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

    #[sqlx::test(migrations = "./migrations")]
    async fn apply_deletes_expired_evaluations_labels_reviews_and_audit(pool: sqlx::PgPool) {
        let evaluation_id = Uuid::new_v4();
        let review_id = Uuid::new_v4();
        let label_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO trust_risk_evaluations (
                id, producer, assessment_key, operation_class, request_fingerprint,
                score, band, recommendation, feature_version, rule_set_version, expires_at
            ) VALUES (
                $1, 'billing-checkout-fixture', $2, 'payment.checkout', $3,
                10, 'low', 'allow', 'features-v1', 'baseline-v1',
                clock_timestamp() - INTERVAL '1 day'
            )
            "#,
        )
        .bind(evaluation_id)
        .bind(format!("retention-eval-{evaluation_id}"))
        .bind(vec![2_u8; 32])
        .execute(&pool)
        .await
        .expect("evaluation");

        sqlx::query(
            r#"
            INSERT INTO trust_risk_review_cases (
                id, evaluation_id, state, expires_at
            ) VALUES ($1, $2, 'open', clock_timestamp() - INTERVAL '1 day')
            "#,
        )
        .bind(review_id)
        .bind(evaluation_id)
        .execute(&pool)
        .await
        .expect("review");

        sqlx::query(
            r#"
            INSERT INTO trust_risk_labels (
                id, schema_version, producer, evaluation_id, kind, source_class,
                source_id, confidence, knowledge_at, mapping_version, fingerprint, expires_at
            ) VALUES (
                $1, 1, 'billing-checkout-fixture', $2, 1, 3, 'product:expired',
                0.5, clock_timestamp(), 'map-v1', $3, clock_timestamp() - INTERVAL '1 day'
            )
            "#,
        )
        .bind(label_id)
        .bind(evaluation_id)
        .bind(vec![6_u8; 32])
        .execute(&pool)
        .await
        .expect("label");

        sqlx::query(
            r#"
            INSERT INTO trust_risk_audit_events (
                action, actor, reason, target_type, target_id, expires_at
            ) VALUES (
                'rules.stage', 'operator:ada', 'expired audit', 'rule_set', 'v1',
                clock_timestamp() - INTERVAL '1 day'
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("audit");

        let result = apply(&pool).await.expect("retention apply");
        assert_eq!(result.labels, 1);
        assert_eq!(result.reviews, 1);
        assert_eq!(result.evaluations, 1);
        assert_eq!(result.audit, 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn retention_run_exits_when_shutdown_flag_is_set(pool: sqlx::PgPool) {
        use tokio::sync::watch;

        use super::super::run;

        let state = crate::grpc_test_support::state(pool);
        let (tx, rx) = watch::channel(false);
        let handle = tokio::spawn(run(state, rx));
        tx.send(true).expect("signal shutdown");
        handle.await.expect("retention task joins");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn retention_run_exits_when_shutdown_sender_drops(pool: sqlx::PgPool) {
        use tokio::sync::watch;

        use super::super::run;

        let state = crate::grpc_test_support::state(pool);
        let (tx, rx) = watch::channel(false);
        let handle = tokio::spawn(run(state, rx));
        drop(tx);
        handle.await.expect("retention task joins");
    }
}
