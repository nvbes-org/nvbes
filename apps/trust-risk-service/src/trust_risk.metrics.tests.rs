use std::{collections::BTreeMap, net::SocketAddr, time::Duration};

use axum::body::Body;
use tower::ServiceExt;

use super::{assessment, init_otlp, install, projection, router, signal};
use crate::{
    app::TrustRiskState,
    config::{ProducerPolicy, RetentionConfig, TrustRiskConfig},
};

const TOKEN: &str = "development-trust-risk-token-32-value";

fn config() -> TrustRiskConfig {
    TrustRiskConfig {
        environment: "test".to_string(),
        sentry_dsn: None,
        sentry_traces_sample_rate: 0.0,
        otlp_endpoint: None,
        otlp_authorization_header: None,
        database_url: "postgres://localhost/nvbes_trust_risk".to_string(),
        bind_addr: "127.0.0.1:3050".parse::<SocketAddr>().unwrap(),
        producers: BTreeMap::from([(
            "billing-checkout-fixture".to_string(),
            ProducerPolicy {
                producer: "billing-checkout-fixture".to_string(),
                token: TOKEN.to_string(),
                signal_prefixes: vec!["network.".to_string()],
                can_assess: true,
                can_label: false,
            },
        )]),
        operators: BTreeMap::new(),
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

fn state() -> TrustRiskState {
    let db = crate::database::connect_lazy("postgres://localhost/nvbes_trust_risk").unwrap();
    TrustRiskState::new(config(), db)
}

#[test]
fn otlp_init_without_endpoint_is_noop() {
    let guard = init_otlp(&config()).expect("init");
    drop(guard);
}

#[test]
fn otlp_init_with_endpoint_configures_provider() {
    let mut cfg = config();
    cfg.otlp_endpoint = Some("https://otlp.example.com:443".to_string());
    cfg.otlp_authorization_header = Some("Basic dXNlcjp0b2tlbg==".to_string());
    let guard = init_otlp(&cfg).expect("init with endpoint");
    signal("billing-checkout-fixture", "network", "accepted");
    assessment("allow", "ok", Duration::from_millis(25));
    projection("projected", 2);
    drop(guard);
}

#[tokio::test]
async fn prometheus_install_and_metrics_route_require_bearer_token() {
    let handle = install();
    signal("billing-checkout-fixture", "network", "accepted");
    assert!(!handle.render().is_empty());

    let app = router(state());
    let unauthorized = app
        .clone()
        .oneshot(
            axum::http::Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthorized.status(), axum::http::StatusCode::UNAUTHORIZED);

    let authorized = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/metrics")
                .header("authorization", format!("Bearer {TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(authorized.status(), axum::http::StatusCode::OK);
    signal("billing-checkout-fixture", "network", "duplicate");
    assessment("review", "ok", Duration::from_millis(10));
    projection("failed", 1);
}
