use super::*;

pub(super) fn evaluation_output(evaluation: RiskEvaluation) -> HashMap<String, Value> {
    let mut output = HashMap::new();
    output.insert("status".to_string(), Value::Number(200.into()));
    output.insert(
        "evaluation_id".to_string(),
        Value::String(evaluation.evaluation_id),
    );
    output.insert("score".to_string(), Value::Number(evaluation.score.into()));
    output.insert(
        "band".to_string(),
        Value::String(band_name(evaluation.band)),
    );
    output.insert(
        "recommendation".to_string(),
        Value::String(recommendation_name(evaluation.recommendation)),
    );
    output.insert(
        "feature_version".to_string(),
        Value::String(evaluation.feature_version),
    );
    output.insert(
        "rule_set_version".to_string(),
        Value::String(evaluation.rule_set_version),
    );
    output.insert("duplicate".to_string(), Value::Bool(evaluation.duplicate));
    output.insert(
        "evaluated_at".to_string(),
        Value::String(timestamp_rfc3339(evaluation.evaluated_at)),
    );
    output.insert(
        "expires_at".to_string(),
        Value::String(timestamp_rfc3339(evaluation.expires_at)),
    );
    output.insert(
        "reasons".to_string(),
        Value::Array(evaluation.reasons.into_iter().map(reason_to_json).collect()),
    );
    output
}

pub(super) fn reason_to_json(reason: RiskReason) -> Value {
    json!({
        "code": reason.code,
        "parameters": reason.parameters,
    })
}

pub(super) fn band_name(value: i32) -> String {
    match RiskBand::try_from(value) {
        Ok(RiskBand::Low) => "low",
        Ok(RiskBand::Elevated) => "elevated",
        Ok(RiskBand::High) => "high",
        Ok(RiskBand::Critical) => "critical",
        _ => "unspecified",
    }
    .to_string()
}

pub(super) fn recommendation_name(value: i32) -> String {
    match RiskRecommendation::try_from(value) {
        Ok(RiskRecommendation::Allow) => "allow",
        Ok(RiskRecommendation::Challenge) => "challenge",
        Ok(RiskRecommendation::Review) => "review",
        Ok(RiskRecommendation::Deny) => "deny",
        _ => "unspecified",
    }
    .to_string()
}

pub(super) fn parse_timestamp(value: Option<&Value>) -> Result<Option<Timestamp>, KeywordError> {
    let Some(value) = value else {
        return Ok(Some(now_timestamp()));
    };
    match value {
        Value::Number(n) => {
            let seconds = n.as_i64().ok_or_else(|| {
                KeywordError::InvalidParams("timestamps must be whole seconds".to_string())
            })?;
            Ok(Some(Timestamp { seconds, nanos: 0 }))
        }
        Value::String(s) => {
            let parsed = chrono::DateTime::parse_from_rfc3339(s).map_err(|_| {
                KeywordError::InvalidParams(format!(
                    "timestamp '{s}' is not a valid RFC 3339 string"
                ))
            })?;
            Ok(Some(Timestamp {
                seconds: parsed.timestamp(),
                nanos: parsed.timestamp_subsec_nanos() as i32,
            }))
        }
        _ => Err(KeywordError::InvalidParams(
            "timestamp must be a number (unix seconds) or RFC 3339 string".to_string(),
        )),
    }
}

pub(super) fn now_timestamp() -> Timestamp {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock before unix epoch");
    Timestamp {
        seconds: now.as_secs() as i64,
        nanos: now.subsec_nanos() as i32,
    }
}

pub(super) fn timestamp_rfc3339(timestamp: Option<Timestamp>) -> String {
    match timestamp {
        Some(timestamp) => {
            chrono::DateTime::from_timestamp(timestamp.seconds, timestamp.nanos as u32)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
        }
        None => "1970-01-01T00:00:00Z".to_string(),
    }
}

pub(super) fn parse_enum_i32(
    value: Option<&Value>,
    default: i32,
    names: &[(&str, i32)],
    what: &str,
) -> Result<i32, KeywordError> {
    let Some(value) = value else {
        return Ok(default);
    };
    if value.is_null() {
        return Ok(default);
    }
    if let Some(number) = value.as_i64() {
        let number = number as i32;
        if names.iter().any(|(_, value)| *value == number) {
            return Ok(number);
        }
        return Err(KeywordError::InvalidParams(format!(
            "{what}: unknown enum value {number}"
        )));
    }
    let name = value
        .as_str()
        .ok_or_else(|| {
            KeywordError::InvalidParams(format!("{what}: enum must be a name or number"))
        })?
        .trim()
        .replace('-', "_")
        .to_lowercase();
    names
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, value)| *value)
        .ok_or_else(|| KeywordError::InvalidParams(format!("{what}: unknown enum name '{name}'")))
}

pub(super) fn string_field(
    object: &Map<String, Value>,
    key: &str,
    default: impl FnOnce() -> String,
) -> String {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(default)
}

pub(super) fn optional_string_field(object: &Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::to_string)
}

pub(super) fn required_string_field(
    object: &Map<String, Value>,
    key: &str,
) -> Result<String, KeywordError> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| KeywordError::InvalidParams(format!("missing required field: {key}")))
}

pub(super) fn required_string_field_str(
    params: &HashMap<String, Value>,
    key: &str,
) -> Result<String, KeywordError> {
    params
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| KeywordError::InvalidParams(format!("missing required param: {key}")))
}

pub(super) fn uint_field(object: &Map<String, Value>, key: &str, default: u64) -> u64 {
    object.get(key).and_then(Value::as_u64).unwrap_or(default)
}

pub(super) fn required_u64_field(value: &Value, what: &str) -> Result<u64, KeywordError> {
    value
        .as_u64()
        .ok_or_else(|| KeywordError::InvalidParams(format!("{what}: expected an integer")))
}

pub(super) fn required_f64_field(
    object: &Map<String, Value>,
    key: &str,
) -> Result<f64, KeywordError> {
    object
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| KeywordError::InvalidParams(format!("missing required field: {key}")))
}

pub(super) fn map_client_error(error: TrustRiskClientError) -> KeywordError {
    match error {
        TrustRiskClientError::Invalid => KeywordError::Execution(
            "trust/risk service rejected the request as invalid".to_string(),
        ),
        TrustRiskClientError::Conflict => KeywordError::Execution(
            "trust/risk request conflicts with an existing idempotency key".to_string(),
        ),
        TrustRiskClientError::Unauthorized => {
            KeywordError::Execution("trust/risk service rejected the caller".to_string())
        }
        TrustRiskClientError::Unavailable => {
            KeywordError::Execution("trust/risk service is unavailable".to_string())
        }
        TrustRiskClientError::Protocol => {
            KeywordError::Execution("trust/risk service returned an invalid response".to_string())
        }
        TrustRiskClientError::Configuration(message) => KeywordError::InvalidParams(message),
    }
}
