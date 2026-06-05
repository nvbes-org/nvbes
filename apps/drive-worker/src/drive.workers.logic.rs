use std::time::Duration;
use uuid::Uuid;

use super::types::StorageObjectType;

pub fn retry_backoff(attempt_count: i32) -> Duration {
    let seconds = 2_i64.pow(attempt_count.min(10) as u32);
    Duration::from_secs(seconds as u64)
}

pub fn storage_object_type_as_str(object_type: StorageObjectType) -> &'static str {
    match object_type {
        StorageObjectType::File => "file",
        StorageObjectType::Folder => "folder",
    }
}

pub fn payload_uuid(payload: &serde_json::Value, field: &str) -> anyhow::Result<Uuid> {
    payload
        .get(field)
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| anyhow::anyhow!("payload missing required uuid field: {field}"))
}

#[cfg(test)]
mod tests {
    use super::payload_uuid;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn parses_uuid_payload_fields() {
        let user_id = Uuid::new_v4();
        let payload = json!({ "user_id": user_id.to_string() });

        let parsed = payload_uuid(&payload, "user_id").expect("uuid field should parse");

        assert_eq!(parsed, user_id);
    }

    #[test]
    fn rejects_missing_uuid_fields() {
        let payload = json!({});

        let error = payload_uuid(&payload, "user_id").expect_err("missing field should fail");

        assert!(
            error
                .to_string()
                .contains("payload missing required uuid field: user_id")
        );
    }
}
