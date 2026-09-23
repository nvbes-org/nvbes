use super::ReviewState;

#[test]
fn review_state_machine_is_explicit_and_resolution_requires_a_label() {
    assert!(ReviewState::Open.permits(ReviewState::InReview, false));
    assert!(ReviewState::Open.permits(ReviewState::InReview, true));
    assert!(!ReviewState::Open.permits(ReviewState::Resolved, true));
    assert!(!ReviewState::Open.permits(ReviewState::Inconclusive, false));
    assert!(!ReviewState::InReview.permits(ReviewState::Resolved, false));
    assert!(ReviewState::InReview.permits(ReviewState::Resolved, true));
    assert!(ReviewState::InReview.permits(ReviewState::Inconclusive, false));
    assert!(!ReviewState::InReview.permits(ReviewState::Open, true));
    assert!(!ReviewState::Resolved.permits(ReviewState::Open, true));
    assert!(!ReviewState::Inconclusive.permits(ReviewState::InReview, true));
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
                "  ",
            )
            .await,
            Err(ReviewError::InvalidInput)
        ));
        let long_reason = "x".repeat(301);
        assert!(matches!(
            transition(
                &pool,
                Uuid::new_v4(),
                ReviewState::InReview,
                None,
                "operator-ada",
                &long_reason,
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

    #[sqlx::test(migrations = "./migrations")]
    async fn transition_rejects_illegal_moves_and_resolves_with_label(pool: sqlx::PgPool) {
        let evaluation_id = seed_evaluation(&pool).await;
        let mut tx = pool.begin().await.expect("tx");
        let case_id = ensure_case_in_transaction(&mut tx, evaluation_id, 400)
            .await
            .expect("open case");
        tx.commit().await.expect("commit");

        assert!(matches!(
            transition(
                &pool,
                case_id,
                ReviewState::Resolved,
                None,
                "operator-ada",
                "skip straight to resolved",
            )
            .await,
            Err(ReviewError::InvalidTransition)
        ));

        transition(
            &pool,
            case_id,
            ReviewState::InReview,
            Some("operator-ada".into()),
            "operator-ada",
            "start investigation",
        )
        .await
        .expect("in review");

        assert!(matches!(
            transition(
                &pool,
                case_id,
                ReviewState::Resolved,
                None,
                "operator-ada",
                "resolve without label",
            )
            .await,
            Err(ReviewError::InvalidTransition)
        ));

        let label_id = Uuid::new_v4();
        sqlx::query(
            r#"
            INSERT INTO trust_risk_labels (
                id, schema_version, producer, evaluation_id, review_case_id, kind,
                source_class, source_id, confidence, actor, knowledge_at,
                evidence_reference, mapping_version, fingerprint, expires_at
            ) VALUES (
                $1, 1, 'backoffice-service', $2, $3, 2, 1, 'review:resolve',
                0.95, 'operator-ada', clock_timestamp(), 'case:resolve',
                'human-review-v1', $4, clock_timestamp() + INTERVAL '400 days'
            )
            "#,
        )
        .bind(label_id)
        .bind(evaluation_id)
        .bind(case_id)
        .bind(vec![11_u8; 32])
        .execute(&pool)
        .await
        .expect("label");
        sqlx::query(
            "INSERT INTO trust_risk_canonical_labels (evaluation_id, label_id) VALUES ($1, $2)",
        )
        .bind(evaluation_id)
        .bind(label_id)
        .execute(&pool)
        .await
        .expect("canonical");

        let resolved = transition(
            &pool,
            case_id,
            ReviewState::Resolved,
            None,
            "operator-ada",
            "confirmed fraud",
        )
        .await
        .expect("resolved");
        assert_eq!(resolved.state, ReviewState::Resolved);
        assert_eq!(resolved.assigned_to.as_deref(), Some("operator-ada"));
    }
}
