use nvbes_trust_risk::proto::nvbes::trust_risk::v1::LabelSourceClass;

use super::authority_rank;

#[test]
fn canonical_precedence_excludes_heuristics() {
    assert!(
        authority_rank(LabelSourceClass::Human)
            > authority_rank(LabelSourceClass::AuthoritativeExternal)
    );
    assert!(
        authority_rank(LabelSourceClass::AuthoritativeExternal)
            > authority_rank(LabelSourceClass::VerifiedProduct)
    );
    assert_eq!(authority_rank(LabelSourceClass::Heuristic), None);
    assert_eq!(authority_rank(LabelSourceClass::Unspecified), None);
}

#[cfg(feature = "database-tests")]
mod database {
    use nvbes_trust_risk::{label::LabelAssertion, proto::nvbes::trust_risk::v1 as pb};
    use uuid::Uuid;

    use super::super::{LabelPersistenceError, fingerprint, persist_label};

    async fn seed_evaluation(pool: &sqlx::PgPool, evaluation_id: Uuid) {
        sqlx::query(
            r#"
            INSERT INTO trust_risk_evaluations (
                id, producer, assessment_key, operation_class, request_fingerprint,
                score, band, recommendation, feature_version, rule_set_version, expires_at
            ) VALUES (
                $1, 'billing-checkout-fixture', $2, 'payment.checkout', $3,
                70, 'high', 'review', 'features-v1', 'baseline-v1',
                clock_timestamp() + INTERVAL '400 days'
            )
            "#,
        )
        .bind(evaluation_id)
        .bind(format!("key-{}", evaluation_id))
        .bind(vec![5_u8; 32])
        .execute(pool)
        .await
        .expect("evaluation");
    }

    fn label_wire(evaluation_id: Uuid, label_id: Uuid) -> pb::RiskLabel {
        pb::RiskLabel {
            label_id: label_id.to_string(),
            schema_version: 1,
            producer: "backoffice-service".to_string(),
            evaluation_id: evaluation_id.to_string(),
            review_case_id: None,
            kind: pb::RiskLabelKind::ConfirmedFraud.into(),
            source_class: pb::LabelSourceClass::Human.into(),
            source_id: "review:case-coverage".to_string(),
            confidence: 0.91,
            actor: Some("operator:ada".to_string()),
            knowledge_at: Some(prost_types::Timestamp {
                seconds: chrono::Utc::now().timestamp(),
                nanos: 0,
            }),
            evidence_reference: Some("case:coverage".to_string()),
            mapping_version: "human-review-v1".to_string(),
            corrects_label_id: None,
        }
    }

    #[test]
    fn fingerprint_changes_with_optional_fields() {
        let evaluation_id = Uuid::new_v4();
        let label_id = Uuid::new_v4();
        let base = LabelAssertion::try_from(label_wire(evaluation_id, label_id)).unwrap();
        let mut without_evidence = label_wire(evaluation_id, label_id);
        without_evidence.evidence_reference = None;
        let stripped = LabelAssertion::try_from(without_evidence).unwrap();
        assert_ne!(fingerprint(&base), fingerprint(&stripped));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn persist_label_is_idempotent_and_sets_canonical(pool: sqlx::PgPool) {
        let evaluation_id = Uuid::new_v4();
        let label_id = Uuid::new_v4();
        seed_evaluation(&pool, evaluation_id).await;
        let wire = label_wire(evaluation_id, label_id);
        let assertion = LabelAssertion::try_from(wire).expect("label");

        let first = persist_label(&pool, &assertion, 400).await.expect("insert");
        assert!(!first.duplicate);
        let second = persist_label(&pool, &assertion, 400).await.expect("replay");
        assert!(second.duplicate);
        assert_eq!(first.id, second.id);

        let canonical: Uuid = sqlx::query_scalar(
            "SELECT label_id FROM trust_risk_canonical_labels WHERE evaluation_id = $1",
        )
        .bind(evaluation_id)
        .fetch_one(&pool)
        .await
        .expect("canonical");
        assert_eq!(canonical, label_id);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn persist_label_rejects_missing_correction_target(pool: sqlx::PgPool) {
        let evaluation_id = Uuid::new_v4();
        let label_id = Uuid::new_v4();
        seed_evaluation(&pool, evaluation_id).await;
        let mut wire = label_wire(evaluation_id, label_id);
        wire.corrects_label_id = Some(Uuid::new_v4().to_string());
        let assertion = LabelAssertion::try_from(wire).expect("label");
        assert!(matches!(
            persist_label(&pool, &assertion, 400).await,
            Err(LabelPersistenceError::InvalidCorrection)
        ));
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn persist_label_rejects_fingerprint_conflict_and_accepts_correction(pool: sqlx::PgPool) {
        let evaluation_id = Uuid::new_v4();
        let original_id = Uuid::new_v4();
        seed_evaluation(&pool, evaluation_id).await;
        let original =
            LabelAssertion::try_from(label_wire(evaluation_id, original_id)).expect("original");
        persist_label(&pool, &original, 400)
            .await
            .expect("insert original");

        let mut conflicting = label_wire(evaluation_id, original_id);
        conflicting.confidence = 0.42;
        let conflict = LabelAssertion::try_from(conflicting).expect("conflict label");
        assert!(matches!(
            persist_label(&pool, &conflict, 400).await,
            Err(LabelPersistenceError::Conflict)
        ));

        let correction_id = Uuid::new_v4();
        let mut correction = label_wire(evaluation_id, correction_id);
        correction.kind = pb::RiskLabelKind::FalsePositive.into();
        correction.corrects_label_id = Some(original_id.to_string());
        correction.source_id = "review:correction".to_string();
        let correction = LabelAssertion::try_from(correction).expect("correction");
        let receipt = persist_label(&pool, &correction, 400)
            .await
            .expect("correction insert");
        assert!(!receipt.duplicate);

        let canonical: Option<Uuid> = sqlx::query_scalar(
            "SELECT label_id FROM trust_risk_canonical_labels WHERE evaluation_id = $1",
        )
        .bind(evaluation_id)
        .fetch_one(&pool)
        .await
        .ok();
        assert_eq!(canonical, Some(correction_id));
    }
}
