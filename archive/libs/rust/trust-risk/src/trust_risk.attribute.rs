use std::collections::{BTreeMap, HashMap};

use crate::{proto::nvbes::trust_risk::v1 as pb, signal::SignalError};

#[derive(Debug, Clone, PartialEq)]
pub enum Attribute {
    String(String),
    Signed(i64),
    Unsigned(u64),
    Bool(bool),
    Decimal(f64),
    Opaque(Vec<u8>),
}

pub(crate) fn convert_attributes(
    family: &str,
    values: HashMap<String, pb::AttributeValue>,
) -> Result<BTreeMap<String, Attribute>, SignalError> {
    values
        .into_iter()
        .map(|(name, value)| convert_attribute(family, name, value))
        .collect()
}

fn convert_attribute(
    family: &str,
    name: String,
    value: pb::AttributeValue,
) -> Result<(String, Attribute), SignalError> {
    if !allowed_attribute(family, &name) {
        return Err(SignalError::InvalidAttribute);
    }
    let attribute = match value.value.ok_or(SignalError::InvalidAttribute)? {
        pb::attribute_value::Value::StringValue(value) if valid_string(&value) => {
            Attribute::String(value)
        }
        pb::attribute_value::Value::SignedValue(value) => Attribute::Signed(value),
        pb::attribute_value::Value::UnsignedValue(value) => Attribute::Unsigned(value),
        pb::attribute_value::Value::BoolValue(value) => Attribute::Bool(value),
        pb::attribute_value::Value::DecimalValue(value) if value.is_finite() => {
            Attribute::Decimal(value)
        }
        pb::attribute_value::Value::OpaqueValue(value) if value.len() <= 256 => {
            Attribute::Opaque(value)
        }
        _ => return Err(SignalError::InvalidAttribute),
    };
    if matches!(name.as_str(), "risk_score" | "provider_risk_score") && !score_is_valid(&attribute)
    {
        return Err(SignalError::InvalidAttribute);
    }
    if name == "confidence" && !confidence_is_valid(&attribute) {
        return Err(SignalError::InvalidAttribute);
    }
    Ok((name, attribute))
}

fn allowed_attribute(family: &str, name: &str) -> bool {
    match family {
        "network" => matches!(name, "risk_score" | "network_kind"),
        "automation" => matches!(name, "confidence" | "detected"),
        "payment" => matches!(name, "outcome" | "provider_risk_score"),
        "reputation" => matches!(name, "confidence" | "trusted"),
        "identity" => matches!(name, "outcome" | "risk_score"),
        "velocity" => matches!(name, "count" | "window_seconds"),
        "usage" | "sharing" | "exfiltration" => matches!(name, "count" | "risk_score"),
        _ => false,
    }
}

fn valid_string(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn score_is_valid(value: &Attribute) -> bool {
    match value {
        Attribute::Unsigned(value) => *value <= 100,
        Attribute::Signed(value) => (0..=100).contains(value),
        Attribute::Decimal(value) => (0.0..=100.0).contains(value),
        _ => false,
    }
}

fn confidence_is_valid(value: &Attribute) -> bool {
    matches!(value, Attribute::Decimal(value) if (0.0..=1.0).contains(value))
}
