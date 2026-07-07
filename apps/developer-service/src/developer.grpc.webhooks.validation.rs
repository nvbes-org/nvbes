use tonic::Status;
use url::Url;

use crate::grpc::service_status::non_empty;

pub fn validate_url(value: String) -> Result<String, Status> {
    let value = non_empty(value, "url")?;
    let parsed_url = Url::parse(&value)
        .map_err(|_| Status::invalid_argument("Webhook endpoint URL must be absolute"))?;
    if !matches!(parsed_url.scheme(), "http" | "https") {
        return Err(Status::invalid_argument(
            "Webhook endpoint URL must use http or https",
        ));
    }
    Ok(value)
}

pub fn optional_url(value: String) -> Result<Option<String>, Status> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        validate_url(value).map(Some)
    }
}

pub fn validate_events(events: Vec<String>) -> Result<Vec<String>, Status> {
    if events.is_empty() {
        return Err(Status::invalid_argument(
            "At least one webhook event is required",
        ));
    }
    let mut unique = Vec::new();
    for event in events {
        let event = event.trim();
        if !matches!(
            event,
            "client.created" | "login.failed" | "session.revoked" | "user.created"
        ) {
            return Err(Status::invalid_argument("Unsupported webhook event type"));
        }
        if !unique.iter().any(|existing| existing == event) {
            unique.push(event.to_string());
        }
    }
    unique.sort();
    Ok(unique)
}

pub fn optional_non_empty(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub fn ensure_replayable(status: &str) -> Result<(), Status> {
    if matches!(status, "failed" | "pending") {
        Ok(())
    } else {
        Err(Status::failed_precondition(
            "Only failed or pending webhook deliveries can be replayed",
        ))
    }
}

pub fn last4(value: &str) -> String {
    value
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}
