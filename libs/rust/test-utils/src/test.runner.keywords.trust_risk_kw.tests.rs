use super::*;

fn signal_param(signal: Value) -> HashMap<String, Value> {
    let mut params = HashMap::new();
    params.insert("signals".to_string(), Value::Array(vec![signal]));
    params
}

fn minimal_signal() -> Map<String, Value> {
    let mut signal = Map::new();
    signal.insert(
        "producer".to_string(),
        Value::String("billing-v1".to_string()),
    );
    signal.insert(
        "signal_kind".to_string(),
        Value::String("payment.declined".to_string()),
    );
    signal.insert(
        "subjects".to_string(),
        Value::Array(vec![json!({
            "kind": "tenant",
            "namespace": "acme",
            "opaque_id": "tnt_123",
        })]),
    );
    signal
}

#[test]
fn parses_full_signal_with_defaults() {
    // single-signal convenience path (top-level keys)
    let mut params = HashMap::new();
    for (key, value) in minimal_signal() {
        params.insert(key, value);
    }
    params.insert("scope".to_string(), Value::String("tenant".to_string()));
    let map = collect_single_signal(&params);
    let signal = signal_from_value(&Value::Object(map)).unwrap();
    assert!(Uuid::parse_str(&signal.signal_id).is_ok());
    assert_eq!(signal.schema_version, 1);
    assert_eq!(signal.producer, "billing-v1");
    assert_eq!(signal.signal_kind, "payment.declined");
    assert_eq!(signal.partition_key, "billing-v1");
    assert_eq!(signal.scope, DataScope::Tenant as i32);
    assert_eq!(signal.subjects.len(), 1);
    assert_eq!(signal.subjects[0].kind, SubjectKind::Tenant as i32);
    assert_eq!(signal.subjects[0].namespace, "acme");
    assert_eq!(signal.subjects[0].opaque_id, "tnt_123");
    assert_eq!(signal.subjects[0].scope, DataScope::Tenant as i32);
    assert!(signal.attributes.is_empty());
    assert!(signal.occurred_at.is_some());
}

#[test]
fn parses_array_of_signals() {
    let mut params = HashMap::new();
    let mut second = minimal_signal();
    second.insert(
        "signal_kind".to_string(),
        Value::String("login.failed".to_string()),
    );
    params.insert(
        "signals".to_string(),
        Value::Array(vec![Value::Object(minimal_signal()), Value::Object(second)]),
    );
    let signals = parse_signals(&params).unwrap();
    assert_eq!(signals.len(), 2);
    assert_eq!(signals[0].signal_kind, "payment.declined");
    assert_eq!(signals[1].signal_kind, "login.failed");
}

#[test]
fn rejects_missing_subjects() {
    let mut signal = minimal_signal();
    signal.remove("subjects");
    let params = signal_param(Value::Object(signal));
    assert!(parse_signals(&params).is_err());
}

#[test]
fn converts_attributes_to_proto() {
    let mut signal = minimal_signal();
    signal.insert(
        "attributes".to_string(),
        json!({
            "outcome": "declined",
            "risk_score": 12,
            "confidence": 0.9,
            "detected": true,
        }),
    );
    let signal = signal_from_value(&Value::Object(signal)).unwrap();
    assert_eq!(signal.attributes.len(), 4);
    let outcome = signal.attributes.get("outcome").unwrap();
    assert!(matches!(
        outcome.value,
        Some(attribute_value::Value::StringValue(_))
    ));
    let risk_score = signal.attributes.get("risk_score").unwrap();
    assert!(matches!(
        risk_score.value,
        Some(attribute_value::Value::UnsignedValue(12))
    ));
    let confidence = signal.attributes.get("confidence").unwrap();
    assert!(matches!(
        confidence.value,
        Some(attribute_value::Value::DecimalValue(value)) if (value - 0.9).abs() < 1e-9
    ));
    let detected = signal.attributes.get("detected").unwrap();
    assert!(matches!(
        detected.value,
        Some(attribute_value::Value::BoolValue(true))
    ));
}

#[test]
fn parses_assessment_request() {
    let mut params = HashMap::new();
    params.insert(
        "producer".to_string(),
        Value::String("billing-v1".to_string()),
    );
    params.insert(
        "assessment_key".to_string(),
        Value::String("pay_123".to_string()),
    );
    params.insert(
        "operation_class".to_string(),
        Value::String("payment.charge".to_string()),
    );
    params.insert(
        "subjects".to_string(),
        Value::Array(vec![json!({
            "kind": "principal",
            "namespace": "acme",
            "opaque_id": "usr_456",
        })]),
    );
    let request = parse_assessment(&params).unwrap();
    assert_eq!(request.producer, "billing-v1");
    assert_eq!(request.assessment_key, "pay_123");
    assert_eq!(request.operation_class, "payment.charge");
    assert_eq!(request.subjects.len(), 1);
    assert_eq!(request.subjects[0].kind, SubjectKind::Principal as i32);
    assert!(request.instantaneous_signals.is_empty());
    assert!(request.context.is_none());
}

#[test]
fn parses_label() {
    let label = json!({
        "producer": "operator-console",
        "evaluation_id": "eval_1",
        "kind": "confirmed_fraud",
        "source_class": "human",
        "source_id": "review-42",
        "confidence": 0.99,
        "actor": "operator@example.com",
    });
    let parsed = label_from_value(&label).unwrap();
    assert!(Uuid::parse_str(&parsed.label_id).is_ok());
    assert_eq!(parsed.schema_version, 1);
    assert_eq!(parsed.producer, "operator-console");
    assert_eq!(parsed.evaluation_id, "eval_1");
    assert_eq!(parsed.kind, RiskLabelKind::ConfirmedFraud as i32);
    assert_eq!(parsed.source_class, LabelSourceClass::Human as i32);
    assert_eq!(parsed.source_id, "review-42");
    assert_eq!(parsed.mapping_version, "v1");
    assert!((parsed.confidence - 0.99).abs() < 1e-9);
    assert_eq!(parsed.actor.as_deref(), Some("operator@example.com"));
}

#[test]
fn parses_enums_from_name_or_number() {
    let names: &[(&str, i32)] = &[
        ("tenant", DataScope::Tenant as i32),
        ("regional", DataScope::Regional as i32),
    ];
    let value = Value::String("tenant".to_string());
    assert_eq!(parse_enum_i32(Some(&value), 0, names, "scope").unwrap(), 1);
    let value = Value::Number(2.into());
    assert_eq!(parse_enum_i32(Some(&value), 0, names, "scope").unwrap(), 2);
    assert_eq!(
        parse_enum_i32(None, DataScope::Regional as i32, names, "scope").unwrap(),
        2
    );
    let value = Value::String("bogus".to_string());
    assert!(parse_enum_i32(Some(&value), 0, names, "scope").is_err());
}

#[test]
fn evaluation_output_uses_str_names() {
    let evaluation = RiskEvaluation {
        evaluation_id: "eval_abc".to_string(),
        score: 78,
        band: RiskBand::High as i32,
        recommendation: RiskRecommendation::Review as i32,
        reasons: vec![RiskReason {
            code: "velocity.spike".to_string(),
            parameters: HashMap::from([("count".to_string(), "42".to_string())]),
        }],
        feature_version: "v3".to_string(),
        rule_set_version: "r5".to_string(),
        evaluated_at: Some(now_timestamp()),
        expires_at: None,
        duplicate: false,
    };
    let output = evaluation_output(evaluation);
    assert_eq!(output.get("score").unwrap(), &Value::Number(78.into()));
    assert_eq!(
        output.get("band").unwrap(),
        &Value::String("high".to_string())
    );
    assert_eq!(
        output.get("recommendation").unwrap(),
        &Value::String("review".to_string())
    );
    assert_eq!(
        output.get("reasons").unwrap().as_array().unwrap()[0]["code"],
        Value::String("velocity.spike".to_string())
    );
}

#[test]
fn maps_client_errors_to_keyword_errors() {
    assert!(matches!(
        map_client_error(TrustRiskClientError::Unavailable),
        KeywordError::Execution(_)
    ));
    assert!(matches!(
        map_client_error(TrustRiskClientError::Configuration("x".to_string())),
        KeywordError::InvalidParams(_)
    ));
}
