use std::{collections::BTreeMap, net::SocketAddr, time::Duration};

use super::{APP_NAME, capture_operation, smoke};
use crate::config::{RetentionConfig, TrustRiskConfig};

fn config() -> TrustRiskConfig {
    TrustRiskConfig {
        environment: "test".to_string(),
        sentry_dsn: None,
        sentry_traces_sample_rate: 0.0,
        otlp_endpoint: None,
        otlp_authorization_header: None,
        database_url: "postgres://localhost/nvbes_trust_risk".to_string(),
        bind_addr: "127.0.0.1:3050".parse::<SocketAddr>().unwrap(),
        producers: BTreeMap::new(),
        operators: BTreeMap::new(),
        metrics_token: "development-trust-risk-token-32-value".to_string(),
        retention: RetentionConfig {
            signals_days: 30,
            evaluations_days: 400,
            labels_days: 400,
            reviews_days: 400,
            audit_days: 730,
        },
        projection_heartbeat_max_age: Duration::from_secs(30),
    }
}

#[test]
fn smoke_reports_service_identity_without_sentry() {
    let result = smoke(&config());
    assert_eq!(result.app_name, APP_NAME);
    assert_eq!(result.environment, "test");
    assert!(!result.configured);
}

#[test]
fn capture_operation_accepts_database_errors_without_panicking() {
    let config = config();
    let error = sqlx::Error::PoolClosed;
    capture_operation(&config, "label.persist", &error);
}
