use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainEventEnvelope {
    pub event_id: Uuid,
    pub event_type: String,
    pub event_version: u16,
    pub tenant_id: Uuid,
    pub region_id: String,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: Uuid,
    pub idempotency_key: String,
    pub payload: Value,
}

#[derive(Debug, Clone)]
pub struct DomainEventInput {
    pub event_type: String,
    pub event_version: u16,
    pub tenant_id: Uuid,
    pub region_id: String,
    pub correlation_id: Uuid,
    pub idempotency_key: String,
    pub payload: Value,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainEventError {
    #[error("event_type must use dotted lowercase notation")]
    InvalidEventType,
    #[error("event_version must be greater than zero")]
    InvalidVersion,
    #[error("region_id is required")]
    MissingRegion,
    #[error("idempotency_key is required")]
    MissingIdempotencyKey,
    #[error("payload must be a JSON object")]
    InvalidPayload,
}

impl DomainEventEnvelope {
    pub fn new(input: DomainEventInput) -> Result<Self, DomainEventError> {
        validate_event_type(&input.event_type)?;
        if input.event_version == 0 {
            return Err(DomainEventError::InvalidVersion);
        }
        if input.region_id.trim().is_empty() {
            return Err(DomainEventError::MissingRegion);
        }
        if input.idempotency_key.trim().is_empty() {
            return Err(DomainEventError::MissingIdempotencyKey);
        }
        if !input.payload.is_object() {
            return Err(DomainEventError::InvalidPayload);
        }

        Ok(Self {
            event_id: Uuid::new_v4(),
            event_type: input.event_type,
            event_version: input.event_version,
            tenant_id: input.tenant_id,
            region_id: input.region_id,
            occurred_at: Utc::now(),
            correlation_id: input.correlation_id,
            idempotency_key: input.idempotency_key,
            payload: input.payload,
        })
    }
}

fn validate_event_type(value: &str) -> Result<(), DomainEventError> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() < 3 {
        return Err(DomainEventError::InvalidEventType);
    }
    if parts.iter().any(|part| {
        part.is_empty()
            || !part
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    }) {
        return Err(DomainEventError::InvalidEventType);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_input() -> DomainEventInput {
        DomainEventInput {
            event_type: "identity.user.created".to_string(),
            event_version: 1,
            tenant_id: Uuid::new_v4(),
            region_id: "eu-fr".to_string(),
            correlation_id: Uuid::new_v4(),
            idempotency_key: "signup:tenant:user".to_string(),
            payload: json!({ "user_id": Uuid::new_v4() }),
        }
    }

    #[test]
    fn builds_versioned_domain_event_envelope() {
        let event = DomainEventEnvelope::new(valid_input()).expect("event input must be valid");

        assert_eq!(event.event_type, "identity.user.created");
        assert_eq!(event.event_version, 1);
        assert_eq!(event.region_id, "eu-fr");
        assert!(event.payload.is_object());
    }

    #[test]
    fn rejects_unversioned_or_untyped_events() {
        let mut input = valid_input();
        input.event_version = 0;
        assert_eq!(
            DomainEventEnvelope::new(input).expect_err("zero version must fail"),
            DomainEventError::InvalidVersion
        );

        let mut input = valid_input();
        input.event_type = "IdentityCreated".to_string();
        assert_eq!(
            DomainEventEnvelope::new(input).expect_err("invalid event type must fail"),
            DomainEventError::InvalidEventType
        );
    }
}
