use std::{collections::BTreeMap, net::SocketAddr, time::Duration};

use tonic::metadata::MetadataMap;

use super::{constant_time_eq, operator, producer};
use crate::config::{OperatorPolicy, ProducerPolicy, RetentionConfig, TrustRiskConfig};

const TOKEN: &str = "producer-token-with-at-least-32-characters";

fn config() -> TrustRiskConfig {
    TrustRiskConfig {
        environment: "test".to_string(),
        database_url: "postgres://localhost/trust_risk_test".to_string(),
        bind_addr: "127.0.0.1:3050".parse::<SocketAddr>().unwrap(),
        producers: BTreeMap::from([(
            "identity-service".to_string(),
            ProducerPolicy {
                producer: "identity-service".to_string(),
                token: TOKEN.to_string(),
                signal_prefixes: vec!["identity.".to_string()],
                can_assess: true,
                can_label: false,
            },
        )]),
        operators: BTreeMap::from([(
            "operator:ada".to_string(),
            OperatorPolicy {
                actor: "operator:ada".to_string(),
                token: TOKEN.to_string(),
                permissions: vec!["evaluation:read".to_string()],
            },
        )]),
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

fn metadata(token: &str) -> MetadataMap {
    let mut metadata = MetadataMap::new();
    metadata.insert("authorization", format!("Bearer {token}").parse().unwrap());
    metadata
}

#[test]
fn bearer_authentication_and_acls_are_exact() {
    let config = config();
    let policy = producer(&metadata(TOKEN), &config, "identity-service").unwrap();
    assert!(policy.can_assess);
    assert!(policy.permits_signal("identity.login"));
    assert!(
        producer(
            &metadata("different-token-with-at-least-32-characters"),
            &config,
            "identity-service"
        )
        .is_err()
    );
    assert!(operator(&metadata(TOKEN), &config, "operator:ada", "evaluation:read").is_ok());
    assert!(operator(&metadata(TOKEN), &config, "operator:ada", "rules:write").is_err());
}

#[test]
fn token_comparison_checks_length_and_content() {
    assert!(constant_time_eq(TOKEN.as_bytes(), TOKEN.as_bytes()));
    assert!(!constant_time_eq(TOKEN.as_bytes(), b"short"));
    assert!(!constant_time_eq(
        TOKEN.as_bytes(),
        b"producer-token-with-at-least-32-characterS"
    ));
}
