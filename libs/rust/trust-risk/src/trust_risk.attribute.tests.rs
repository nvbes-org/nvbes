use std::collections::HashMap;

use crate::{
    attribute::{Attribute, convert_attributes},
    proto::nvbes::trust_risk::v1 as pb,
    signal::SignalError,
};

fn value(inner: pb::attribute_value::Value) -> pb::AttributeValue {
    pb::AttributeValue { value: Some(inner) }
}

fn convert(
    family: &str,
    name: &str,
    inner: pb::attribute_value::Value,
) -> Result<Attribute, SignalError> {
    let values = HashMap::from([(name.to_string(), value(inner))]);
    convert_attributes(family, values).map(|mut converted| converted.remove(name).unwrap())
}

#[test]
fn every_typed_variant_round_trips_through_conversion() {
    use pb::attribute_value::Value;

    assert_eq!(
        convert("network", "network_kind", Value::StringValue("vpn".into())),
        Ok(Attribute::String("vpn".into()))
    );
    assert_eq!(
        convert("identity", "risk_score", Value::SignedValue(42)),
        Ok(Attribute::Signed(42))
    );
    assert_eq!(
        convert("network", "risk_score", Value::UnsignedValue(82)),
        Ok(Attribute::Unsigned(82))
    );
    assert_eq!(
        convert("reputation", "trusted", Value::BoolValue(true)),
        Ok(Attribute::Bool(true))
    );
    assert_eq!(
        convert("automation", "confidence", Value::DecimalValue(0.5)),
        Ok(Attribute::Decimal(0.5))
    );
    assert_eq!(
        convert("velocity", "count", Value::OpaqueValue(vec![1, 2, 3])),
        Ok(Attribute::Opaque(vec![1, 2, 3]))
    );
}

#[test]
fn attribute_names_are_allowlisted_per_family() {
    use pb::attribute_value::Value;

    for (family, name) in [
        ("network", "network_kind"),
        ("automation", "detected"),
        ("payment", "outcome"),
        ("reputation", "trusted"),
        ("identity", "outcome"),
        ("velocity", "window_seconds"),
        ("usage", "count"),
        ("sharing", "count"),
        ("exfiltration", "count"),
    ] {
        assert!(
            convert(family, name, Value::BoolValue(true)).is_ok(),
            "{family}.{name} must be allowed"
        );
    }

    assert_eq!(
        convert("network", "outcome", Value::BoolValue(true)),
        Err(SignalError::InvalidAttribute)
    );
    assert_eq!(
        convert("unknown", "count", Value::BoolValue(true)),
        Err(SignalError::InvalidAttribute)
    );
}

#[test]
fn malformed_payloads_are_rejected() {
    use pb::attribute_value::Value;

    let empty = HashMap::from([("count".to_string(), pb::AttributeValue { value: None })]);
    assert_eq!(
        convert_attributes("velocity", empty),
        Err(SignalError::InvalidAttribute)
    );
    assert_eq!(
        convert("network", "network_kind", Value::StringValue(String::new())),
        Err(SignalError::InvalidAttribute)
    );
    assert_eq!(
        convert(
            "network",
            "network_kind",
            Value::StringValue("data centre".into())
        ),
        Err(SignalError::InvalidAttribute)
    );
    assert_eq!(
        convert("automation", "confidence", Value::DecimalValue(f64::NAN)),
        Err(SignalError::InvalidAttribute)
    );
    assert_eq!(
        convert("velocity", "count", Value::OpaqueValue(vec![0; 257])),
        Err(SignalError::InvalidAttribute)
    );
}

#[test]
fn scores_stay_within_their_declared_range() {
    use pb::attribute_value::Value;

    assert_eq!(
        convert("network", "risk_score", Value::UnsignedValue(101)),
        Err(SignalError::InvalidAttribute)
    );
    assert_eq!(
        convert("identity", "risk_score", Value::SignedValue(-1)),
        Err(SignalError::InvalidAttribute)
    );
    assert_eq!(
        convert("payment", "provider_risk_score", Value::DecimalValue(100.5)),
        Err(SignalError::InvalidAttribute)
    );
    assert_eq!(
        convert("network", "risk_score", Value::StringValue("high".into())),
        Err(SignalError::InvalidAttribute)
    );
}

#[test]
fn confidence_must_be_a_decimal_probability() {
    use pb::attribute_value::Value;

    assert_eq!(
        convert("automation", "confidence", Value::DecimalValue(1.5)),
        Err(SignalError::InvalidAttribute)
    );
    assert_eq!(
        convert("reputation", "confidence", Value::UnsignedValue(1)),
        Err(SignalError::InvalidAttribute)
    );
}
