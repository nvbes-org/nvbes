use crate::http::error::AppError;

pub type AuthorizationDetails = Vec<serde_json::Value>;

pub fn parse_authorization_details(raw: Option<&str>) -> Result<AuthorizationDetails, AppError> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(Vec::new());
    };

    let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| {
        AppError::bad_request(
            "invalid_authorization_details",
            "authorization_details must be a JSON array of objects.",
        )
    })?;

    validate_authorization_details_value(value)
}

pub fn parse_authorization_details_value(
    value: Option<&serde_json::Value>,
) -> Result<AuthorizationDetails, AppError> {
    match value {
        Some(value) => validate_authorization_details_value(value.clone()),
        None => Ok(Vec::new()),
    }
}

fn validate_authorization_details_value(
    value: serde_json::Value,
) -> Result<AuthorizationDetails, AppError> {
    let serde_json::Value::Array(details) = value else {
        return Err(invalid_details());
    };

    for detail in &details {
        let serde_json::Value::Object(object) = detail else {
            return Err(invalid_details());
        };
        let Some(detail_type) = object.get("type").and_then(|value| value.as_str()) else {
            return Err(AppError::bad_request(
                "invalid_authorization_details",
                "Each authorization detail must include a non-empty type.",
            ));
        };
        if detail_type.trim().is_empty() {
            return Err(AppError::bad_request(
                "invalid_authorization_details",
                "Each authorization detail must include a non-empty type.",
            ));
        }
    }

    Ok(details)
}

fn invalid_details() -> AppError {
    AppError::bad_request(
        "invalid_authorization_details",
        "authorization_details must be a JSON array of objects.",
    )
}

#[cfg(test)]
mod tests {
    use super::parse_authorization_details;

    #[test]
    fn parses_rfc9396_authorization_details() {
        let details = parse_authorization_details(Some(
            r#"[{"type":"drive:file","file_id":"X","actions":["read"]}]"#,
        ))
        .expect("valid RAR should parse");

        assert_eq!(details.len(), 1);
        assert_eq!(details[0]["type"], "drive:file");
        assert_eq!(details[0]["actions"][0], "read");
    }

    #[test]
    fn rejects_detail_without_type() {
        let error = parse_authorization_details(Some(r#"[{"actions":["read"]}]"#))
            .expect_err("missing type should fail");

        assert_eq!(error.code, "invalid_authorization_details");
    }
}
