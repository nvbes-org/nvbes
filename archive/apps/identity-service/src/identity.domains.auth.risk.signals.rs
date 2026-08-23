use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::http::error::AppError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextRiskSignals {
    pub new_ip: bool,
    pub new_device: bool,
    pub unusual_country: bool,
    pub score_delta: f64,
}

pub fn evaluate_context_signals(
    ip_address: Option<&str>,
    country: Option<&str>,
    device_fingerprint_hash: Option<&str>,
    known_ips: &[String],
    known_countries: &[String],
    known_device_hashes: &[String],
) -> ContextRiskSignals {
    let new_ip = ip_address.is_some_and(|ip| !known_ips.iter().any(|known| known == ip));
    let unusual_country =
        country.is_some_and(|country| !known_countries.iter().any(|known| known == country));
    let new_device = device_fingerprint_hash
        .is_some_and(|hash| !known_device_hashes.iter().any(|known| known == hash));

    let mut score_delta = 0.0;
    if new_ip {
        score_delta += 10.0;
    }
    if new_device {
        score_delta += 10.0;
    }
    if unusual_country {
        score_delta += 10.0;
    }

    ContextRiskSignals {
        new_ip,
        new_device,
        unusual_country,
        score_delta,
    }
}

pub async fn evaluate_login_context_signals(
    db: &PgPool,
    principal_id: Uuid,
    ip_address: Option<&str>,
    country: Option<&str>,
    device_fingerprint_hash: Option<&str>,
) -> Result<ContextRiskSignals, AppError> {
    let rows = sqlx::query(
        r#"
        SELECT DISTINCT
          ip_address::text AS ip_address,
          metadata->>'country' AS country,
          metadata->>'device_fingerprint_hash' AS device_fingerprint_hash
        FROM risk_events
        WHERE principal_id = $1
          AND event_type = 'login_success'
          AND created_at >= NOW() - INTERVAL '90 days'
        ORDER BY ip_address::text NULLS LAST
        LIMIT 200
        "#,
    )
    .bind(principal_id)
    .fetch_all(db)
    .await?;

    let mut known_ips = Vec::new();
    let mut known_countries = Vec::new();
    let mut known_device_hashes = Vec::new();

    for row in rows {
        if let Some(value) = row.get::<Option<String>, _>("ip_address") {
            known_ips.push(value);
        }
        if let Some(value) = row.get::<Option<String>, _>("country") {
            known_countries.push(value);
        }
        if let Some(value) = row.get::<Option<String>, _>("device_fingerprint_hash") {
            known_device_hashes.push(value);
        }
    }

    Ok(evaluate_context_signals(
        ip_address,
        country,
        device_fingerprint_hash,
        &known_ips,
        &known_countries,
        &known_device_hashes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_signals_detect_new_ip_device_and_country() {
        let signals = evaluate_context_signals(
            Some("203.0.113.10"),
            Some("FR"),
            Some("device-a"),
            &["198.51.100.1".to_string()],
            &["US".to_string()],
            &["device-b".to_string()],
        );

        assert!(signals.new_ip);
        assert!(signals.new_device);
        assert!(signals.unusual_country);
        assert_eq!(signals.score_delta, 30.0);
    }

    #[test]
    fn context_signals_ignore_missing_values() {
        let signals = evaluate_context_signals(None, None, None, &[], &[], &[]);

        assert!(!signals.new_ip);
        assert!(!signals.new_device);
        assert!(!signals.unusual_country);
        assert_eq!(signals.score_delta, 0.0);
    }
}
