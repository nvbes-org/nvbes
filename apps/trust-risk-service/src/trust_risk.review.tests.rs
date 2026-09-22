use super::ReviewState;

#[test]
fn review_state_machine_is_explicit_and_resolution_requires_a_label() {
    assert!(ReviewState::Open.permits(ReviewState::InReview, false));
    assert!(!ReviewState::Open.permits(ReviewState::Resolved, true));
    assert!(!ReviewState::InReview.permits(ReviewState::Resolved, false));
    assert!(ReviewState::InReview.permits(ReviewState::Resolved, true));
    assert!(ReviewState::InReview.permits(ReviewState::Inconclusive, false));
    assert!(!ReviewState::Resolved.permits(ReviewState::Open, true));
}

#[test]
fn review_state_labels_are_stable() {
    assert_eq!(ReviewState::Open.as_str(), "open");
    assert_eq!(ReviewState::InReview.as_str(), "in_review");
    assert_eq!(ReviewState::Resolved.as_str(), "resolved");
    assert_eq!(ReviewState::Inconclusive.as_str(), "inconclusive");
}

#[cfg(feature = "database-tests")]
mod database {
    use uuid::Uuid;

    use super::super::{ReviewError, ReviewState, ensure_case_in_transaction, transition};

    async fn seed_evaluation(pool: &sqlx::PgPool) -> Uuid {
        let evaluation_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO trust_risk_evaluations (
                id, producer, assessment_key, operation_class, request_fingerprint,
                score, band, recommendation, feature_version, rule_set_version, expires_at
            ) VALUES (
                $1, 'billing-checkout-fixture', $2, 'payment.checkout', $3,
                80, 'critical', 'review', 'features-v1', 'baseline-v1',
                clock_timestamp() + INTERVAL '400 days'
            )
            "#,
        )
        .bind(evaluation_id)
        .bind(format!("review-{}", evaluation_id))
        .bind(vec![9_u8; 32])
        .execute(pool)
        .await
        .expect("evaluation");
        evaluation_id
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn ensure_case_and_transition_to_inconclusive(pool: sqlx::PgPool) {
        let evaluation_id = seed_evaluation(&pool).await;
        let mut tx = pool.begin().await.expect("tx");
        let case_id = ensure_case_in_transaction(&mut tx, evaluation_id, 400)
            .await
            .expect("open case");
        let again = ensure_case_in_transaction(&mut tx, evaluation_id, 400)
            .await
            .expect("idempotent open");
        assert_eq!(case_id, again);
        tx.commit().await.expect("commit");

        let updated = transition(
            &pool,
            case_id,
            ReviewState::InReview,
            Some("operator-ada".into()),
            "operator-ada",
            "start investigation",
        )
        .await
        .expect("in review");
        assert_eq!(updated.state, ReviewState::InReview);

        let closed = transition(
            &pool,
            case_id,
            ReviewState::Inconclusive,
            None,
            "operator-ada",
            "insufficient evidence",
        )
        .await
        .expect("inconclusive");
        assert_eq!(closed.state, ReviewState::Inconclusive);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn transition_rejects_invalid_input_and_unknown_case(pool: sqlx::PgPool) {
        assert!(matches!(
            transition(
                &pool,
                Uuid::new_v4(),
                ReviewState::InReview,
                None,
                "ab",
                "too-short-actor",
            )
            .await,
            Err(ReviewError::InvalidInput)
        ));
        assert!(matches!(
            transition(
                &pool,
                Uuid::new_v4(),
                ReviewState::InReview,
                None,
                "operator-ada",
                "missing case",
            )
            .await,
            Err(ReviewError::NotFound)
        ));
    }
}
