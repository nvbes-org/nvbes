use chrono::{DateTime, Utc};
use nvbes_billing::usage::UsageEvent;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveUsageEventInput {
    pub tenant_id: Uuid,
    pub workspace_id: Uuid,
    pub meter_code: String,
    pub quantity: i64,
    pub unit: String,
    pub occurred_at: DateTime<Utc>,
    pub source_operation_id: String,
}

pub fn build_drive_usage_event(input: DriveUsageEventInput) -> UsageEvent {
    UsageEvent {
        tenant_id: input.tenant_id,
        workspace_id: Some(input.workspace_id),
        meter_code: input.meter_code,
        quantity: input.quantity,
        unit: input.unit,
        occurred_at: input.occurred_at,
        source: "drive-api".to_string(),
        idempotency_key: input.source_operation_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drive_usage_event_uses_operation_as_idempotency_key() {
        let event = build_drive_usage_event(DriveUsageEventInput {
            tenant_id: Uuid::nil(),
            workspace_id: Uuid::nil(),
            meter_code: "storage_gb_month".to_string(),
            quantity: 1,
            unit: "gb_month".to_string(),
            occurred_at: DateTime::from_timestamp(1_735_689_600, 0).unwrap(),
            source_operation_id: "snapshot-1".to_string(),
        });

        assert_eq!(event.source, "drive-api");
        assert_eq!(event.idempotency_key, "snapshot-1");
    }
}
