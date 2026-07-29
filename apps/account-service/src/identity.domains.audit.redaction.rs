use serde_json::Value;

const REDACTED: &str = "[REDACTED]";
const MAX_STRING_LENGTH: usize = 2_048;
const MAX_DEPTH: usize = 12;

const SENSITIVE_KEYS: &[&str] = &[
    "access_token",
    "authorization",
    "client_secret",
    "cookie",
    "credential",
    "device_cookie",
    "email",
    "file_name",
    "filename",
    "jwt",
    "object_key",
    "password",
    "recovery_code",
    "refresh_token",
    "secret",
    "secret_base32",
    "session_token",
    "token",
    "totp",
];

pub(super) fn redact_audit_metadata(metadata: Value) -> Value {
    redact_value(metadata, 0)
}

fn redact_value(value: Value, depth: usize) -> Value {
    if depth >= MAX_DEPTH {
        return Value::String("[TRUNCATED]".to_string());
    }

    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| {
                    let value = if is_sensitive_key(&key) {
                        Value::String(REDACTED.to_string())
                    } else {
                        redact_value(value, depth + 1)
                    };
                    (key, value)
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|value| redact_value(value, depth + 1))
                .collect(),
        ),
        Value::String(value) => Value::String(redact_string(value)),
        other => other,
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace('-', "_");
    SENSITIVE_KEYS.iter().any(|sensitive| {
        normalized == *sensitive
            || normalized.ends_with(&format!("_{sensitive}"))
            || normalized.starts_with(&format!("{sensitive}_"))
    })
}

fn redact_string(value: String) -> String {
    if looks_like_authorization(&value) || looks_like_jwt(&value) || looks_like_email(&value) {
        return REDACTED.to_string();
    }

    if value.chars().count() <= MAX_STRING_LENGTH {
        return value;
    }

    value.chars().take(MAX_STRING_LENGTH).collect()
}

fn looks_like_authorization(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("bearer ") || lower.starts_with("basic ")
}

fn looks_like_jwt(value: &str) -> bool {
    let mut segments = value.split('.');
    matches!(
        (segments.next(), segments.next(), segments.next(), segments.next()),
        (Some(header), Some(payload), Some(signature), None)
            if header.len() >= 8 && payload.len() >= 8 && signature.len() >= 8
    )
}

fn looks_like_email(value: &str) -> bool {
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    !local.is_empty() && domain.contains('.') && !domain.contains(char::is_whitespace)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{MAX_STRING_LENGTH, redact_audit_metadata};

    #[test]
    fn redacts_nested_secrets_and_direct_identifiers() {
        let redacted = redact_audit_metadata(json!({
            "status": "revoked",
            "client_secret": "never-log-me",
            "changes": {
                "primary_email": "person@example.com",
                "authorization": "Bearer abc.def.ghi",
            },
        }));

        assert_eq!(redacted["status"], "revoked");
        assert_eq!(redacted["client_secret"], "[REDACTED]");
        assert_eq!(redacted["changes"]["primary_email"], "[REDACTED]");
        assert_eq!(redacted["changes"]["authorization"], "[REDACTED]");
    }

    #[test]
    fn redacts_secret_shaped_values_even_under_neutral_keys() {
        let redacted = redact_audit_metadata(json!({
            "value": "Bearer opaque-token",
            "subject": "person@example.com",
            "jwt_value": "abcdefgh.ijklmnop.qrstuvwx",
        }));

        assert_eq!(redacted["value"], "[REDACTED]");
        assert_eq!(redacted["subject"], "[REDACTED]");
        assert_eq!(redacted["jwt_value"], "[REDACTED]");
    }

    #[test]
    fn bounds_untrusted_metadata_strings() {
        let redacted = redact_audit_metadata(json!({
            "reason": "a".repeat(MAX_STRING_LENGTH + 100),
        }));

        assert_eq!(
            redacted["reason"]
                .as_str()
                .expect("reason must stay a string")
                .len(),
            MAX_STRING_LENGTH
        );
    }
}
