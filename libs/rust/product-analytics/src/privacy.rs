use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use sha2::Sha256;
use uuid::Uuid;

use crate::event::AnalyticsProperties;

type HmacSha256 = Hmac<Sha256>;

const ALLOWED_EVENTS: &[&str] = &[
    "auth.signup_completed",
    "auth.email_verified",
    "workspace.created",
    "file.upload_started",
    "file.upload_completed",
    "activation.first_file_uploaded",
    "share_link.created",
    "activation.first_share_link_created",
    "member.invited",
    "activation.first_member_invited",
    "activation.workspace_activated",
    "billing.checkout_started",
    "billing.subscription_activated",
    "billing.trial_ending",
    "billing.payment_failed",
    "retention.workspace_active_weekly",
    "retention.workspace_retained_30d",
];

const ALLOWED_PROPERTIES: &[&str] = &[
    "$groups",
    "country",
    "plan_code",
    "status",
    "workspace_id",
    "workspace_type",
    "user_id",
    "file_count",
    "size_bytes_bucket",
    "upload_count",
    "share_link_count",
    "member_count",
];

pub(crate) fn is_allowed_event(name: &str) -> bool {
    ALLOWED_EVENTS.contains(&name)
}

pub(crate) fn sanitize_properties(properties: AnalyticsProperties) -> AnalyticsProperties {
    let mut sanitized = AnalyticsProperties::new();
    for (key, value) in properties {
        if matches!(key.as_str(), "workspace_id" | "user_id") {
            continue;
        }
        if !ALLOWED_PROPERTIES.contains(&key.as_str()) || is_sensitive_key(&key) {
            continue;
        }
        if let Some(value) = sanitize_value(value) {
            sanitized.insert(key, value);
        }
    }
    sanitized
}

pub(crate) fn add_pseudonymous_context(
    properties: &mut AnalyticsProperties,
    salt: &str,
    user_id: Option<Uuid>,
    workspace_id: Option<Uuid>,
) {
    if let Some(user_id) = user_id {
        properties.insert(
            "user_id".to_string(),
            json!(pseudonymous_id(salt, "usr", user_id)),
        );
    }
    if let Some(workspace_id) = workspace_id {
        let group_id = pseudonymous_id(salt, "wks", workspace_id);
        properties.insert("workspace_id".to_string(), json!(group_id.clone()));
        properties.insert("$groups".to_string(), json!({ "workspace": group_id }));
    }
}

pub(crate) fn pseudonymous_id(salt: &str, prefix: &str, id: Uuid) -> String {
    let mut mac =
        HmacSha256::new_from_slice(salt.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(id.as_bytes());
    let bytes = mac.finalize().into_bytes();
    format!("{prefix}_{}", hex_encode(&bytes[..16]))
}

fn sanitize_value(value: Value) -> Option<Value> {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => Some(value),
        Value::String(value) if value.len() <= 200 && !looks_sensitive_value(&value) => {
            Some(Value::String(value))
        }
        _ => None,
    }
}

fn looks_sensitive_value(value: &str) -> bool {
    value.contains('@') || value.matches('.').count() >= 2 || value.len() > 512
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    normalized.contains("email")
        || normalized.contains("name")
        || normalized.contains("filename")
        || normalized.contains("object_key")
        || normalized.contains("token")
        || normalized.contains("secret")
        || normalized.contains("password")
        || normalized.contains("url")
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::properties;

    #[test]
    fn pseudonymous_ids_are_stable_and_prefixed() {
        let id = Uuid::parse_str("018f2f61-4875-7f7a-8bc8-8f70a73d2b1f").unwrap();
        let first = pseudonymous_id("salt", "usr", id);
        let second = pseudonymous_id("salt", "usr", id);

        assert_eq!(first, second);
        assert!(first.starts_with("usr_"));
        assert!(!first.contains(&id.to_string()));
    }

    #[test]
    fn sanitizer_drops_sensitive_properties() {
        let input = properties([
            ("plan_code", json!("team")),
            ("email", json!("user@example.com")),
            ("object_key", json!("workspaces/raw-key")),
            ("workspace_id", json!("raw")),
        ]);

        let output = sanitize_properties(input);

        assert_eq!(output.get("plan_code"), Some(&json!("team")));
        assert!(!output.contains_key("email"));
        assert!(!output.contains_key("object_key"));
        assert!(!output.contains_key("workspace_id"));
    }
}
