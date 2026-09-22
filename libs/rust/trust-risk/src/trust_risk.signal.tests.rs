use chrono::Utc;
use prost_types::Timestamp;

use crate::proto::nvbes::trust_risk::v1::{
    AttributeValue, DataScope, RiskSignal as WireSignal, SubjectKind,
    SubjectReference as WireSubject,
};
use crate::signal::{RiskSignal, SignalError, SubjectReference, validated_name};

fn base_subject() -> WireSubject {
    WireSubject {
        kind: SubjectKind::Principal.into(),
        namespace: "nvbes.identity".to_string(),
        opaque_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d001".to_string(),
        scope: DataScope::Regional.into(),
        tenant_id: Some("018f7f2d-fc7d-7b7a-9f72-3abddda8d001".to_string()),
    }
}

fn base_signal() -> WireSignal {
    let now = Utc::now();
    WireSignal {
        signal_id: "018f7f2d-fc7d-7b7a-9f72-3abddda8d002".to_string(),
        schema_version: 1,
        producer: "identity-service".to_string(),
        signal_kind: "auth.login.failed".to_string(),
        occurred_at: Some(Timestamp {
            seconds: now.timestamp(),
            nanos: 0,
        }),
        partition_key: "tenant:eu-fr".to_string(),
        subjects: vec![base_subject()],
        attributes: std::collections::HashMap::new(),
        context: None,
        scope: DataScope::Regional.into(),
    }
}

#[test]
fn accepts_valid_risk_signal_and_exposes_accessors() {
    let signal = RiskSignal::try_from(base_signal()).expect("valid signal");
    assert_eq!(signal.producer(), "identity-service");
    assert_eq!(signal.kind(), "auth.login.failed");
    assert_eq!(signal.partition_key(), "tenant:eu-fr");
    assert_eq!(signal.subjects().len(), 1);
    assert!(signal.attributes().is_empty());
    assert_eq!(signal.scope(), DataScope::Regional);
    assert!(!signal.id().is_nil());
    assert!(signal.occurred_at() <= Utc::now());
}

#[test]
fn rejects_invalid_signal_id_and_schema() {
    let mut signal = base_signal();
    signal.signal_id = "not-a-uuid".to_string();
    assert_eq!(
        RiskSignal::try_from(signal).unwrap_err(),
        SignalError::InvalidSignalId
    );

    let mut signal = base_signal();
    signal.schema_version = 2;
    assert_eq!(
        RiskSignal::try_from(signal).unwrap_err(),
        SignalError::UnsupportedSchema
    );
}

#[test]
fn rejects_invalid_producer_kind_timestamp_and_partition() {
    let mut signal = base_signal();
    signal.producer = "ab".to_string();
    assert_eq!(
        RiskSignal::try_from(signal).unwrap_err(),
        SignalError::InvalidProducer
    );

    let mut signal = base_signal();
    signal.signal_kind = "authlogin".to_string();
    assert_eq!(
        RiskSignal::try_from(signal).unwrap_err(),
        SignalError::InvalidSignalKind
    );

    let mut signal = base_signal();
    signal.occurred_at = None;
    assert_eq!(
        RiskSignal::try_from(signal).unwrap_err(),
        SignalError::InvalidTimestamp
    );

    let mut signal = base_signal();
    signal.partition_key = "!!".to_string();
    assert_eq!(
        RiskSignal::try_from(signal).unwrap_err(),
        SignalError::InvalidPartitionKey
    );
}

#[test]
fn rejects_empty_subjects_and_invalid_scopes() {
    let mut signal = base_signal();
    signal.subjects.clear();
    assert_eq!(
        RiskSignal::try_from(signal).unwrap_err(),
        SignalError::InvalidSubjects
    );

    let mut signal = base_signal();
    signal.scope = DataScope::Unspecified.into();
    assert_eq!(
        RiskSignal::try_from(signal).unwrap_err(),
        SignalError::InvalidScope
    );

    let mut signal = base_signal();
    signal.scope = DataScope::GlobalDerived.into();
    assert_eq!(
        RiskSignal::try_from(signal).unwrap_err(),
        SignalError::InvalidScope
    );
}

#[test]
fn rejects_too_many_attributes() {
    let mut signal = base_signal();
    for index in 0..33 {
        signal.attributes.insert(
            format!("metric_{index}"),
            AttributeValue {
                value: Some(
                    crate::proto::nvbes::trust_risk::v1::attribute_value::Value::UnsignedValue(1),
                ),
            },
        );
    }
    assert_eq!(
        RiskSignal::try_from(signal).unwrap_err(),
        SignalError::InvalidAttribute
    );
}

#[test]
fn subject_reference_rejects_bad_kind_namespace_and_tenant_scope() {
    let mut subject = base_subject();
    subject.kind = SubjectKind::Unspecified.into();
    assert_eq!(
        SubjectReference::try_from(subject).unwrap_err(),
        SignalError::InvalidSubjectKind
    );

    let mut subject = base_subject();
    subject.namespace = "AB".to_string();
    assert_eq!(
        SubjectReference::try_from(subject).unwrap_err(),
        SignalError::InvalidNamespace
    );

    let mut subject = base_subject();
    subject.opaque_id = "short".to_string();
    assert_eq!(
        SubjectReference::try_from(subject).unwrap_err(),
        SignalError::InvalidSubject
    );

    let mut subject = base_subject();
    subject.scope = DataScope::Tenant.into();
    subject.tenant_id = None;
    assert_eq!(
        SubjectReference::try_from(subject).unwrap_err(),
        SignalError::InvalidTenant
    );

    let mut subject = base_subject();
    subject.tenant_id = Some("bad".to_string());
    assert_eq!(
        SubjectReference::try_from(subject).unwrap_err(),
        SignalError::InvalidTenant
    );
}

#[test]
fn subject_accessors_expose_parsed_fields() {
    let subject = SubjectReference::try_from(base_subject()).expect("valid subject");
    assert_eq!(subject.kind(), SubjectKind::Principal);
    assert_eq!(subject.opaque_id(), "018f7f2d-fc7d-7b7a-9f72-3abddda8d001");
    assert_eq!(subject.scope(), DataScope::Regional);
    assert!(subject.tenant_id().is_some());
}

#[test]
fn validated_name_enforces_length_and_charset() {
    assert_eq!(validated_name("ab", 3, 10, false), None);
    assert_eq!(validated_name("abc", 3, 10, false).as_deref(), Some("abc"));
    assert_eq!(validated_name("ABC", 3, 10, true), None);
    assert_eq!(validated_name("Abc", 3, 10, false).as_deref(), Some("Abc"));
    assert_eq!(
        validated_name(" bad ", 3, 10, false).as_deref(),
        Some("bad")
    );
}
