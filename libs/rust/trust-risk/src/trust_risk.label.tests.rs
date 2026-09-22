use uuid::Uuid;

use crate::{
    label::{LabelAssertion, LabelError},
    proto::nvbes::trust_risk::v1 as pb,
};

const LABEL_ID: &str = "018f7f2d-fc7d-7b7a-9f72-3abddda8d010";
const EVALUATION_ID: &str = "018f7f2d-fc7d-7b7a-9f72-3abddda8d011";
const REVIEW_CASE_ID: &str = "018f7f2d-fc7d-7b7a-9f72-3abddda8d012";
const CORRECTED_ID: &str = "018f7f2d-fc7d-7b7a-9f72-3abddda8d013";

/// Etiquette humaine complete : toutes les branches optionnelles sont peuplees.
fn human_label() -> pb::RiskLabel {
    pb::RiskLabel {
        label_id: LABEL_ID.to_string(),
        schema_version: 1,
        producer: "backoffice-service".to_string(),
        evaluation_id: EVALUATION_ID.to_string(),
        review_case_id: Some(REVIEW_CASE_ID.to_string()),
        kind: pb::RiskLabelKind::ConfirmedFraud.into(),
        source_class: pb::LabelSourceClass::Human.into(),
        source_id: "review:case-42".to_string(),
        confidence: 0.75,
        actor: Some("operator:ada".to_string()),
        knowledge_at: Some(prost_types::Timestamp {
            seconds: 1_787_590_000,
            nanos: 250,
        }),
        evidence_reference: Some("case:018f7f2d".to_string()),
        mapping_version: "human-review-v1".to_string(),
        corrects_label_id: Some(CORRECTED_ID.to_string()),
    }
}

fn error_of(mutate: impl FnOnce(&mut pb::RiskLabel)) -> LabelError {
    let mut label = human_label();
    mutate(&mut label);
    LabelAssertion::try_from(label).expect_err("label must be rejected")
}

#[test]
fn accepted_label_exposes_every_parsed_field() {
    let assertion = LabelAssertion::try_from(human_label()).expect("label is accepted");

    assert_eq!(assertion.id(), Uuid::parse_str(LABEL_ID).unwrap());
    assert_eq!(
        assertion.evaluation_id(),
        Uuid::parse_str(EVALUATION_ID).unwrap()
    );
    assert_eq!(
        assertion.review_case_id(),
        Some(Uuid::parse_str(REVIEW_CASE_ID).unwrap())
    );
    assert_eq!(
        assertion.corrects_label_id(),
        Some(Uuid::parse_str(CORRECTED_ID).unwrap())
    );
    assert_eq!(assertion.producer(), "backoffice-service");
    assert_eq!(assertion.kind(), pb::RiskLabelKind::ConfirmedFraud);
    assert_eq!(assertion.source_class(), pb::LabelSourceClass::Human);
    assert_eq!(assertion.source_id(), "review:case-42");
    assert_eq!(assertion.confidence(), 0.75);
    assert_eq!(assertion.actor(), Some("operator:ada"));
    assert_eq!(assertion.evidence_reference(), Some("case:018f7f2d"));
    assert_eq!(assertion.mapping_version(), "human-review-v1");
    assert_eq!(assertion.knowledge_at().timestamp(), 1_787_590_000);
    assert_eq!(assertion.knowledge_at().timestamp_subsec_nanos(), 250);
}

#[test]
fn blank_optional_fields_are_read_as_absent() {
    let mut label = human_label();
    label.review_case_id = Some("   ".to_string());
    label.actor = Some(String::new());
    label.evidence_reference = Some(" ".to_string());
    label.corrects_label_id = Some(String::new());
    label.source_class = pb::LabelSourceClass::Heuristic.into();

    let assertion = LabelAssertion::try_from(label).expect("blank optionals are tolerated");

    assert_eq!(assertion.review_case_id(), None);
    assert_eq!(assertion.actor(), None);
    assert_eq!(assertion.evidence_reference(), None);
    assert_eq!(assertion.corrects_label_id(), None);
}

#[test]
fn identifiers_must_be_uuids() {
    assert_eq!(
        error_of(|label| label.label_id = "not-a-uuid".to_string()),
        LabelError::InvalidLabelId
    );
    assert_eq!(
        error_of(|label| label.evaluation_id = "not-a-uuid".to_string()),
        LabelError::InvalidEvaluationId
    );
    assert_eq!(
        error_of(|label| label.review_case_id = Some("not-a-uuid".to_string())),
        LabelError::InvalidReviewCaseId
    );
    assert_eq!(
        error_of(|label| label.corrects_label_id = Some("not-a-uuid".to_string())),
        LabelError::InvalidCorrectionId
    );
}

#[test]
fn a_label_cannot_correct_itself() {
    assert_eq!(
        error_of(|label| label.corrects_label_id = Some(LABEL_ID.to_string())),
        LabelError::InvalidCorrectionId
    );
}

#[test]
fn only_schema_version_one_is_accepted() {
    assert_eq!(
        error_of(|label| label.schema_version = 2),
        LabelError::UnsupportedSchema
    );
}

#[test]
fn kind_and_source_class_must_be_known_and_specified() {
    assert_eq!(
        error_of(|label| label.kind = pb::RiskLabelKind::Unspecified.into()),
        LabelError::InvalidKind
    );
    assert_eq!(error_of(|label| label.kind = 99), LabelError::InvalidKind);
    assert_eq!(
        error_of(|label| label.source_class = pb::LabelSourceClass::Unspecified.into()),
        LabelError::InvalidSourceClass
    );
    assert_eq!(
        error_of(|label| label.source_class = 99),
        LabelError::InvalidSourceClass
    );
}

#[test]
fn confidence_must_be_a_finite_probability() {
    assert_eq!(
        error_of(|label| label.confidence = f64::NAN),
        LabelError::InvalidConfidence
    );
    assert_eq!(
        error_of(|label| label.confidence = f64::INFINITY),
        LabelError::InvalidConfidence
    );
    assert_eq!(
        error_of(|label| label.confidence = -0.01),
        LabelError::InvalidConfidence
    );
}

#[test]
fn free_text_fields_reject_unsafe_or_undersized_values() {
    assert_eq!(
        error_of(|label| label.producer = "x".to_string()),
        LabelError::InvalidProducer
    );
    assert_eq!(
        error_of(|label| label.source_id = "review case 42".to_string()),
        LabelError::InvalidSourceId
    );
    assert_eq!(
        error_of(|label| label.actor = Some("operator ada".to_string())),
        LabelError::InvalidActor
    );
    assert_eq!(
        error_of(|label| label.evidence_reference = Some("case 018f".to_string())),
        LabelError::InvalidEvidenceReference
    );
    assert_eq!(
        error_of(|label| label.mapping_version = String::new()),
        LabelError::InvalidMappingVersion
    );
}

#[test]
fn a_knowledge_timestamp_is_mandatory_and_must_be_representable() {
    assert_eq!(
        error_of(|label| label.knowledge_at = None),
        LabelError::InvalidTimestamp
    );
    assert_eq!(
        error_of(|label| {
            label.knowledge_at = Some(prost_types::Timestamp {
                seconds: i64::MAX,
                nanos: 0,
            })
        }),
        LabelError::InvalidTimestamp
    );
}
