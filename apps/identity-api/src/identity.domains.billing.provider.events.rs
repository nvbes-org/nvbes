use nvbes_billing::provider::{ProviderCode, ProviderWebhookEvent};

pub fn provider_event_audit_key(event: &ProviderWebhookEvent) -> String {
    let provider = match event.provider {
        ProviderCode::Stripe => "stripe",
        ProviderCode::Mollie => "mollie",
    };
    format!("{provider}:{}", event.provider_event_id)
}

pub fn can_replay_provider_event(status: &str) -> bool {
    matches!(status, "failed" | "rejected")
}

pub fn provider_status_after_insert(existing_status: Option<&str>) -> &'static str {
    match existing_status {
        Some("processed" | "replayed") => "duplicate",
        Some("failed" | "rejected") => "verified",
        _ => "verified",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_key_is_provider_scoped() {
        let event = ProviderWebhookEvent {
            provider: ProviderCode::Stripe,
            provider_event_id: "evt_1".to_string(),
            event_type: "checkout.session.completed".to_string(),
            payload_hash: "hash".to_string(),
            payload_summary: serde_json::json!({ "type": "checkout.session.completed" }),
            signature_valid: true,
            raw_retention_class: "summary_only".to_string(),
        };

        assert_eq!(provider_event_audit_key(&event), "stripe:evt_1");
    }

    #[test]
    fn only_failed_or_rejected_events_can_replay() {
        assert!(can_replay_provider_event("failed"));
        assert!(can_replay_provider_event("rejected"));
        assert!(!can_replay_provider_event("processed"));
    }

    #[test]
    fn failed_provider_event_is_reverified_on_replay_insert() {
        assert_eq!(provider_status_after_insert(Some("failed")), "verified");
        assert_eq!(provider_status_after_insert(Some("processed")), "duplicate");
        assert_eq!(provider_status_after_insert(None), "verified");
    }
}
