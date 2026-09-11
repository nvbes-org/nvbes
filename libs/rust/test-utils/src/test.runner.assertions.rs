use std::collections::HashMap;

use serde::Deserialize;
use serde_json::Value;

use super::variables::{VariableContext, substitute_value};

#[derive(Debug, thiserror::Error)]
pub enum AssertionError {
    #[error("expected status {expected}, got {actual}")]
    StatusMismatch { expected: u16, actual: u16 },

    #[error("expected body field {field} = {expected:?}, got {actual:?}")]
    BodyFieldMismatch {
        field: String,
        expected: Value,
        actual: Value,
    },

    #[error("expected body field {field} to contain substring {substring:?}")]
    BodySubstringMismatch { field: String, substring: String },

    #[error("expected error containing {expected:?}, got none")]
    ExpectedError { expected: String },

    #[error("unexpected error: {0}")]
    UnexpectedError(String),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExpectBlock {
    #[serde(default, deserialize_with = "deserialize_optional_status")]
    pub status: Option<u16>,
    #[serde(default)]
    pub body: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub contains: Option<HashMap<String, String>>,
}

fn deserialize_optional_status<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Value::deserialize(deserializer)?;
    match value {
        Value::Null => Ok(None),
        Value::Number(n) => n
            .as_u64()
            .and_then(|v| u16::try_from(v).ok())
            .map(Some)
            .ok_or_else(|| serde::de::Error::custom(format!("invalid status number: {n}"))),
        Value::String(s) => match s.as_str() {
            "ok" | "OK" | "200" => Ok(Some(200)),
            "created" | "CREATED" | "201" => Ok(Some(201)),
            "accepted" | "ACCEPTED" | "202" => Ok(Some(202)),
            "no_content" | "NO_CONTENT" | "204" => Ok(Some(204)),
            "bad_request" | "BAD_REQUEST" | "400" => Ok(Some(400)),
            "unauthorized" | "UNAUTHORIZED" | "401" => Ok(Some(401)),
            "forbidden" | "FORBIDDEN" | "403" => Ok(Some(403)),
            "not_found" | "NOT_FOUND" | "404" => Ok(Some(404)),
            "conflict" | "CONFLICT" | "409" => Ok(Some(409)),
            "internal_error" | "INTERNAL_ERROR" | "500" => Ok(Some(500)),
            other => other
                .parse::<u16>()
                .map(Some)
                .map_err(|_| serde::de::Error::custom(format!("unknown status alias: {other}"))),
        },
        _ => Err(serde::de::Error::custom(
            "status must be a number or string",
        )),
    }
}

pub fn validate_assertions(
    expect: &ExpectBlock,
    actual_status: Option<u16>,
    actual_body: &HashMap<String, Value>,
    actual_error: Option<&str>,
    ctx: &VariableContext,
) -> Result<(), Vec<AssertionError>> {
    let mut errors = Vec::new();

    if let Some(expected_status) = expect.status {
        match actual_status {
            Some(actual) if actual != expected_status => {
                errors.push(AssertionError::StatusMismatch {
                    expected: expected_status,
                    actual,
                });
            }
            None => {
                errors.push(AssertionError::StatusMismatch {
                    expected: expected_status,
                    actual: 0,
                });
            }
            _ => {}
        }
    }

    if let Some(ref expected_body) = expect.body {
        for (field, expected_value) in expected_body {
            let resolved_expected = substitute_value(expected_value, ctx);
            match actual_body.get(field.as_str()) {
                Some(actual_value) => {
                    let resolved_actual = substitute_value(actual_value, ctx);
                    if resolved_expected != resolved_actual
                        && resolved_expected != Value::String("*".to_string())
                    {
                        errors.push(AssertionError::BodyFieldMismatch {
                            field: field.clone(),
                            expected: resolved_expected,
                            actual: resolved_actual,
                        });
                    }
                }
                None => {
                    if resolved_expected != Value::Null {
                        errors.push(AssertionError::BodyFieldMismatch {
                            field: field.clone(),
                            expected: resolved_expected,
                            actual: Value::Null,
                        });
                    }
                }
            }
        }
    }

    if let Some(ref expected_error) = expect.error {
        match actual_error {
            Some(actual) if actual.contains(expected_error) => {}
            Some(actual) => {
                errors.push(AssertionError::ExpectedError {
                    expected: expected_error.clone(),
                });
                let _ = actual;
            }
            None => {
                errors.push(AssertionError::ExpectedError {
                    expected: expected_error.clone(),
                });
            }
        }
    } else if let Some(actual) = actual_error {
        errors.push(AssertionError::UnexpectedError(actual.to_string()));
    }

    if let Some(ref expected_contains) = expect.contains {
        for (field, substring) in expected_contains {
            let resolved_sub = substitute(substring, ctx);
            match actual_body.get(field.as_str()) {
                Some(Value::String(s)) => {
                    if !s.contains(resolved_sub.as_str()) {
                        errors.push(AssertionError::BodySubstringMismatch {
                            field: field.clone(),
                            substring: resolved_sub,
                        });
                    }
                }
                Some(actual) => {
                    let s = actual.to_string();
                    if !s.contains(resolved_sub.as_str()) {
                        errors.push(AssertionError::BodySubstringMismatch {
                            field: field.clone(),
                            substring: resolved_sub,
                        });
                    }
                }
                None => {
                    errors.push(AssertionError::BodySubstringMismatch {
                        field: field.clone(),
                        substring: resolved_sub,
                    });
                }
            }
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

use super::variables::substitute;

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> VariableContext {
        VariableContext::new(HashMap::new())
    }

    #[test]
    fn passes_when_no_expect() {
        let expect = ExpectBlock {
            status: None,
            body: None,
            error: None,
            contains: None,
        };
        let result = validate_assertions(&expect, Some(200), &HashMap::new(), None, &ctx());
        assert!(result.is_ok());
    }

    #[test]
    fn validates_status() {
        let expect = ExpectBlock {
            status: Some(200),
            body: None,
            error: None,
            contains: None,
        };
        assert!(validate_assertions(&expect, Some(200), &HashMap::new(), None, &ctx()).is_ok());
        assert!(validate_assertions(&expect, Some(404), &HashMap::new(), None, &ctx()).is_err());
    }

    #[test]
    fn validates_body_field() {
        let mut body = HashMap::new();
        body.insert("ok".to_string(), Value::Bool(true));
        let expect = ExpectBlock {
            status: None,
            body: Some(HashMap::from([("ok".to_string(), Value::Bool(true))])),
            error: None,
            contains: None,
        };
        assert!(validate_assertions(&expect, None, &body, None, &ctx()).is_ok());
    }

    #[test]
    fn wildcard_matches_any() {
        let mut body = HashMap::new();
        body.insert("token".to_string(), Value::String("anything".to_string()));
        let expect = ExpectBlock {
            status: None,
            body: Some(HashMap::from([(
                "token".to_string(),
                Value::String("*".to_string()),
            )])),
            error: None,
            contains: None,
        };
        assert!(validate_assertions(&expect, None, &body, None, &ctx()).is_ok());
    }

    #[test]
    fn validates_error_presence() {
        let expect = ExpectBlock {
            status: None,
            body: None,
            error: Some("not found".to_string()),
            contains: None,
        };
        assert!(
            validate_assertions(
                &expect,
                None,
                &HashMap::new(),
                Some("resource not found"),
                &ctx()
            )
            .is_ok()
        );
        assert!(validate_assertions(&expect, None, &HashMap::new(), None, &ctx()).is_err());
    }
}
