use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::http::error::AppError;

pub const EVENT_TYPE: &str = "identity.principal.registered.v1";
const CURRENT_LEGAL_DOCUMENT_VERSION: &str = "2026-06-26";
const CONSENT_TYPES: [&str; 3] = [
    "terms_of_service",
    "privacy_policy",
    "data_processing_agreement",
];

#[derive(Serialize)]
struct AccountRegistrationProjection {
    event_id: Uuid,
    event_type: &'static str,
    principal_id: Uuid,
    marketing_emails_accepted: bool,
    consent_ip_address: Option<String>,
    consents: Vec<RegistrationConsent>,
    registered_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct RegistrationConsent {
    consent_type: &'static str,
    document_version: &'static str,
}

pub async fn enqueue_tx(
    tx: &mut Transaction<'_, Postgres>,
    principal_id: Uuid,
    marketing_emails_accepted: bool,
    ip_address: Option<&str>,
    registered_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let event_id = Uuid::new_v4();
    let payload = serde_json::to_value(AccountRegistrationProjection {
        event_id,
        event_type: EVENT_TYPE,
        principal_id,
        marketing_emails_accepted,
        consent_ip_address: ip_address.and_then(anonymize_ip),
        consents: CONSENT_TYPES
            .into_iter()
            .map(|consent_type| RegistrationConsent {
                consent_type,
                document_version: CURRENT_LEGAL_DOCUMENT_VERSION,
            })
            .collect(),
        registered_at,
    })
    .map_err(|error| {
        AppError::internal(
            "account_registration_event_invalid",
            format!("Account registration event could not be serialized: {error}"),
        )
    })?;

    sqlx::query(
        r#"
        INSERT INTO identity_outbox_events (
          id, aggregate_type, aggregate_id, event_type, payload, occurred_at
        )
        VALUES ($1, 'principal', $2, $3, $4, $5)
        "#,
    )
    .bind(event_id)
    .bind(principal_id)
    .bind(EVENT_TYPE)
    .bind(payload)
    .bind(registered_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn anonymize_ip(value: &str) -> Option<String> {
    match value.parse::<std::net::IpAddr>().ok()? {
        std::net::IpAddr::V4(address) => {
            let octets = address.octets();
            Some(std::net::Ipv4Addr::new(octets[0], octets[1], octets[2], 0).to_string())
        }
        std::net::IpAddr::V6(address) => {
            let segments = address.segments();
            Some(
                std::net::Ipv6Addr::new(segments[0], segments[1], segments[2], 0, 0, 0, 0, 0)
                    .to_string(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::anonymize_ip;

    #[test]
    fn consent_ip_is_minimized_before_leaving_identity() {
        assert_eq!(anonymize_ip("203.0.113.42").as_deref(), Some("203.0.113.0"));
        assert_eq!(
            anonymize_ip("2001:db8:abcd:12::1").as_deref(),
            Some("2001:db8:abcd::")
        );
        assert_eq!(anonymize_ip("not-an-ip"), None);
    }
}
