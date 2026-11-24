use super::*;

pub(super) fn parse_signals(
    params: &HashMap<String, Value>,
) -> Result<Vec<RiskSignal>, KeywordError> {
    if let Some(Value::Array(signals)) = params.get("signals") {
        signals.iter().map(signal_from_value).collect()
    } else {
        Ok(vec![signal_from_value(&Value::Object(
            collect_single_signal(params),
        ))?])
    }
}

pub(super) fn collect_single_signal(params: &HashMap<String, Value>) -> Map<String, Value> {
    let mut fields = Map::new();
    for key in [
        "signal_id",
        "schema_version",
        "producer",
        "signal_kind",
        "occurred_at",
        "partition_key",
        "subjects",
        "attributes",
        "scope",
        "context",
    ] {
        if let Some(value) = params.get(key) {
            fields.insert(key.to_string(), value.clone());
        }
    }
    fields
}

pub(super) fn signal_from_value(value: &Value) -> Result<RiskSignal, KeywordError> {
    let object = value
        .as_object()
        .ok_or_else(|| KeywordError::InvalidParams("signal must be an object".to_string()))?;
    let signal_id = string_field(object, "signal_id", || Uuid::new_v4().to_string());
    let schema_version = uint_field(object, "schema_version", 1) as u32;
    let producer = required_string_field(object, "producer")?;
    let signal_kind = required_string_field(object, "signal_kind")?;
    let occurred_at = parse_timestamp(object.get("occurred_at"))?;
    let partition_key = string_field(object, "partition_key", || producer.clone());
    let subjects = parse_subjects(object.get("subjects"))?;
    let attributes = parse_attributes(object.get("attributes"))?;
    let scope = parse_enum_i32(
        object.get("scope"),
        DEFAULT_SCOPE,
        &[
            ("tenant", DataScope::Tenant as i32),
            ("regional", DataScope::Regional as i32),
        ],
        "scope",
    )?;
    let context = parse_context(object.get("context"))?;

    Ok(RiskSignal {
        signal_id,
        schema_version,
        producer,
        signal_kind,
        occurred_at,
        partition_key,
        subjects,
        attributes,
        context,
        scope,
    })
}

pub(super) fn parse_subjects(value: Option<&Value>) -> Result<Vec<SubjectReference>, KeywordError> {
    let array = value.and_then(Value::as_array).ok_or_else(|| {
        KeywordError::InvalidParams("subjects must be a non-empty array".to_string())
    })?;
    if array.is_empty() {
        return Err(KeywordError::InvalidParams(
            "subjects must not be empty".to_string(),
        ));
    }
    array.iter().map(subject_from_value).collect()
}

pub(super) fn subject_from_value(value: &Value) -> Result<SubjectReference, KeywordError> {
    let object = value
        .as_object()
        .ok_or_else(|| KeywordError::InvalidParams("subject must be an object".to_string()))?;
    let kind = parse_enum_i32(
        object.get("kind"),
        SubjectKind::Unspecified as i32,
        &[
            ("principal", SubjectKind::Principal as i32),
            ("device", SubjectKind::Device as i32),
            ("network", SubjectKind::Network as i32),
            ("tenant", SubjectKind::Tenant as i32),
            ("workload", SubjectKind::Workload as i32),
            ("anonymous_session", SubjectKind::AnonymousSession as i32),
        ],
        "subject kind",
    )?;
    Ok(SubjectReference {
        kind,
        namespace: required_string_field(object, "namespace")?,
        opaque_id: required_string_field(object, "opaque_id")?,
        scope: parse_enum_i32(
            object.get("scope"),
            DEFAULT_SCOPE,
            &[
                ("tenant", DataScope::Tenant as i32),
                ("regional", DataScope::Regional as i32),
            ],
            "subject scope",
        )?,
        tenant_id: optional_string_field(object, "tenant_id"),
    })
}

pub(super) fn parse_attributes(
    value: Option<&Value>,
) -> Result<HashMap<String, AttributeValue>, KeywordError> {
    let mut attributes = HashMap::new();
    let Some(object) = value else {
        return Ok(attributes);
    };
    let object = object
        .as_object()
        .ok_or_else(|| KeywordError::InvalidParams("attributes must be an object".to_string()))?;
    for (name, attr_value) in object {
        attributes.insert(name.clone(), attribute_value_from_json(attr_value)?);
    }
    Ok(attributes)
}

pub(super) fn attribute_value_from_json(value: &Value) -> Result<AttributeValue, KeywordError> {
    let attribute = match value {
        Value::Null => {
            return Err(KeywordError::InvalidParams(
                "attribute values must not be null".to_string(),
            ));
        }
        Value::Bool(b) => AttributeValue {
            value: Some(attribute_value::Value::BoolValue(*b)),
        },
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                if i < 0 {
                    AttributeValue {
                        value: Some(attribute_value::Value::SignedValue(i)),
                    }
                } else {
                    AttributeValue {
                        value: Some(attribute_value::Value::UnsignedValue(i as u64)),
                    }
                }
            } else if let Some(f) = n.as_f64() {
                AttributeValue {
                    value: Some(attribute_value::Value::DecimalValue(f)),
                }
            } else {
                return Err(KeywordError::InvalidParams(
                    "attribute number is out of range".to_string(),
                ));
            }
        }
        Value::String(s) => AttributeValue {
            value: Some(attribute_value::Value::StringValue(s.clone())),
        },
        Value::Array(bytes) => {
            let mut decoded = Vec::new();
            for byte in bytes {
                decoded.push(required_u64_field(byte, "opaque byte")? as u8);
            }
            AttributeValue {
                value: Some(attribute_value::Value::OpaqueValue(decoded)),
            }
        }
        Value::Object(_) => {
            return Err(KeywordError::InvalidParams(
                "attribute values must be scalar".to_string(),
            ));
        }
    };
    Ok(attribute)
}
