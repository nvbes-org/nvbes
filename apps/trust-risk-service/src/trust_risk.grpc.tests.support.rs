#![allow(dead_code)]

use std::{collections::BTreeMap, net::SocketAddr, time::Duration};

use nvbes_trust_risk::proto::nvbes::trust_risk::v1 as pb;
use sqlx::PgPool;
use tonic::metadata::MetadataMap;

use crate::{
    app::TrustRiskState,
    config::{OperatorPolicy, ProducerPolicy, RetentionConfig, TrustRiskConfig},
};

pub const TOKEN: &str = "development-trust-risk-token-32-value";

pub fn config(database_url: impl Into<String>) -> TrustRiskConfig {
    TrustRiskConfig {
        environment: "test".to_string(),
        sentry_dsn: None,
        sentry_traces_sample_rate: 0.0,
        otlp_endpoint: None,
        otlp_authorization_header: None,
        database_url: database_url.into(),
        bind_addr: "127.0.0.1:3050".parse::<SocketAddr>().unwrap(),
        producers: BTreeMap::from([(
            "billing-checkout-fixture".to_string(),
            ProducerPolicy {
                producer: "billing-checkout-fixture".to_string(),
                token: TOKEN.to_string(),
                signal_prefixes: vec![
                    "network.".to_string(),
                    "payment.".to_string(),
                    "velocity.".to_string(),
                ],
                can_assess: true,
                can_label: true,
            },
        )]),
        operators: BTreeMap::from([(
            "development-operator".to_string(),
            OperatorPolicy {
                actor: "development-operator".to_string(),
                token: TOKEN.to_string(),
                permissions: vec![
                    "evaluation:read".to_string(),
                    "review:write".to_string(),
                    "rules:write".to_string(),
                    "labels:human".to_string(),
                ],
            },
        )]),
        metrics_token: TOKEN.to_string(),
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

pub fn state(pool: PgPool) -> TrustRiskState {
    TrustRiskState::new(config("postgres://trust-risk/test"), pool)
}

pub fn producer_metadata() -> MetadataMap {
    bearer(TOKEN)
}

pub fn operator_metadata() -> MetadataMap {
    bearer(TOKEN)
}

fn bearer(token: &str) -> MetadataMap {
    let mut metadata = MetadataMap::new();
    metadata.insert("authorization", format!("Bearer {token}").parse().unwrap());
    metadata
}

pub fn operator_context() -> pb::OperatorContext {
    pb::OperatorContext {
        caller: "backoffice".to_string(),
        actor: "development-operator".to_string(),
        reason: "coverage exercise".to_string(),
        request_context: None,
    }
}
