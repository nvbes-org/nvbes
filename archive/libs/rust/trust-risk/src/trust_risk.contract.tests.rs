use std::collections::HashMap;

use crate::{
    assessment::Assessment,
    label::{LabelAssertion, LabelError},
    proto::nvbes::trust_risk::v1 as pb,
    signal::{RiskSignal, SignalError},
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

fn signal(attributes: HashMap<String, pb::AttributeValue>) -> pb::RiskSignal {
    pb::RiskSignal {
        signal_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d002".to_string(),
        schema_version: 1,
        producer: "billing-checkout-fixture".to_string(),
        signal_kind: "network.reputation".to_string(),
        occurred_at: Some(prost_types::Timestamp {
            seconds: 1_787_590_000,
            nanos: 0,
        }),
        partition_key: "regional:eu-west:network:example".to_string(),
        subjects: vec![subject()],
        attributes,
        context: None,
        scope: pb::DataScope::Regional.into(),
    }
}

#[test]
fn signal_conversion_accepts_allowlisted_typed_attributes() {
    let attributes = HashMap::from([
        (
            "risk_score".to_string(),
            pb::AttributeValue {
                value: Some(pb::attribute_value::Value::UnsignedValue(82)),
            },
        ),
        (
            "network_kind".to_string(),
            pb::AttributeValue {
                value: Some(pb::attribute_value::Value::StringValue(
                    "datacenter".to_string(),
                )),
            },
        ),
    ]);

    let signal = RiskSignal::try_from(signal(attributes)).unwrap();
    assert_eq!(signal.kind(), "network.reputation");
    assert_eq!(signal.attributes().len(), 2);
}

#[test]
fn signal_conversion_rejects_unknown_attributes() {
    let attributes = HashMap::from([(
        "email".to_string(),
        pb::AttributeValue {
            value: Some(pb::attribute_value::Value::StringValue(
                "ada@example.com".to_string(),
            )),
        },
    )]);

    assert_eq!(
        RiskSignal::try_from(signal(attributes)).unwrap_err(),
        SignalError::InvalidAttribute
    );
}

#[test]
fn assessment_requires_consistent_instantaneous_producers() {
    let mut evidence = signal(HashMap::new());
    evidence.producer = "identity-service".to_string();
    let request = pb::AssessRiskRequest {
        producer: "billing-checkout-fixture".to_string(),
        assessment_key: "checkout:018f7f2d-fc7d-7b7a-9f72".to_string(),
        operation_class: "payment.checkout".to_string(),
        subjects: vec![subject()],
        instantaneous_signals: vec![evidence],
        context: None,
    };

    assert!(Assessment::try_from(request).is_err());
}

#[test]
fn human_labels_require_an_actor_and_bounded_confidence() {
    let label = pb::RiskLabel {
        label_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d003".to_string(),
        schema_version: 1,
        producer: "backoffice-service".to_string(),
        evaluation_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d004".to_string(),
        review_case_id: None,
        kind: pb::RiskLabelKind::FalsePositive.into(),
        source_class: pb::LabelSourceClass::Human.into(),
        source_id: "review:case-42".to_string(),
        confidence: 1.0,
        actor: None,
        knowledge_at: Some(prost_types::Timestamp {
            seconds: 1_787_590_000,
            nanos: 0,
        }),
        evidence_reference: None,
        mapping_version: "human-review-v1".to_string(),
        corrects_label_id: None,
    };

    assert_eq!(
        LabelAssertion::try_from(label.clone()).unwrap_err(),
        LabelError::HumanActorRequired
    );

    let mut excessive_confidence = label;
    excessive_confidence.actor = Some("operator:ada".to_string());
    excessive_confidence.confidence = 1.01;
    assert_eq!(
        LabelAssertion::try_from(excessive_confidence).unwrap_err(),
        LabelError::InvalidConfidence
    );
}
