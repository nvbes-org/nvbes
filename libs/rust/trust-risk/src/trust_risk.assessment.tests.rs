use crate::{
    assessment::{Assessment, AssessmentError},
    proto::nvbes::trust_risk::v1 as pb,
};

fn subject() -> pb::SubjectReference {
    pb::SubjectReference {
        kind: pb::SubjectKind::Principal.into(),
        namespace: "nvbes.identity".to_string(),
        opaque_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d001".to_string(),
        scope: pb::DataScope::Regional.into(),
        tenant_id: None,
    }
}

fn signal(producer: &str) -> pb::RiskSignal {
    pb::RiskSignal {
        signal_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d002".to_string(),
        schema_version: 1,
        producer: producer.to_string(),
        signal_kind: "network.reputation".to_string(),
        occurred_at: Some(prost_types::Timestamp {
            seconds: 1_787_590_000,
            nanos: 0,
        }),
        partition_key: "regional:eu-west:network:example".to_string(),
        subjects: vec![subject()],
        attributes: Default::default(),
        context: None,
        scope: pb::DataScope::Regional.into(),
    }
}

fn request() -> pb::AssessRiskRequest {
    pb::AssessRiskRequest {
        producer: "billing-checkout".to_string(),
        assessment_key: "checkout:018f7f2d-fc7d".to_string(),
        operation_class: "payment.checkout".to_string(),
        subjects: vec![subject()],
        instantaneous_signals: vec![signal("billing-checkout")],
        context: None,
    }
}

#[test]
fn accepts_consistent_assessment_and_exposes_accessors() {
    let assessment = Assessment::try_from(request()).expect("valid assessment");
    assert_eq!(assessment.producer(), "billing-checkout");
    assert_eq!(assessment.key(), "checkout:018f7f2d-fc7d");
    assert_eq!(assessment.operation_class(), "payment.checkout");
    assert_eq!(assessment.subjects().len(), 1);
    assert_eq!(assessment.instantaneous_signals().len(), 1);
}

#[test]
fn rejects_invalid_producer_key_and_operation_class() {
    let mut value = request();
    value.producer = "ab".to_string();
    assert_eq!(
        Assessment::try_from(value).unwrap_err(),
        AssessmentError::InvalidProducer
    );

    let mut value = request();
    value.assessment_key = "short".to_string();
    assert_eq!(
        Assessment::try_from(value).unwrap_err(),
        AssessmentError::InvalidKey
    );

    let mut value = request();
    value.operation_class = "payment".to_string();
    assert_eq!(
        Assessment::try_from(value).unwrap_err(),
        AssessmentError::InvalidOperationClass
    );
}

#[test]
fn rejects_empty_subjects_and_too_many_signals() {
    let mut value = request();
    value.subjects.clear();
    assert_eq!(
        Assessment::try_from(value).unwrap_err(),
        AssessmentError::InvalidSubjects
    );

    let mut value = request();
    value.subjects = (0..17).map(|_| subject()).collect();
    assert_eq!(
        Assessment::try_from(value).unwrap_err(),
        AssessmentError::InvalidSubjects
    );

    let mut value = request();
    value.instantaneous_signals = (0..17).map(|_| signal("billing-checkout")).collect();
    assert_eq!(
        Assessment::try_from(value).unwrap_err(),
        AssessmentError::InvalidSignals
    );
}

#[test]
fn accepts_exact_subject_and_signal_limits() {
    let mut value = request();
    value.subjects = (0..16).map(|_| subject()).collect();
    Assessment::try_from(value).expect("16 subjects must be accepted");

    let mut value = request();
    value.instantaneous_signals = (0..16).map(|_| signal("billing-checkout")).collect();
    Assessment::try_from(value).expect("16 instantaneous signals must be accepted");
}

#[test]
fn rejects_producer_mismatch_on_instantaneous_signals() {
    let mut value = request();
    value.instantaneous_signals = vec![signal("other-producer")];
    assert_eq!(
        Assessment::try_from(value).unwrap_err(),
        AssessmentError::ProducerMismatch
    );
}
