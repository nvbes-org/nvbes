use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};

use nvbes_trust_risk::{
    client::{TrustRiskClient, TrustRiskClientConfig, TrustRiskClientError},
    proto::nvbes::trust_risk::v1::{
        self as pb, trust_risk_assessment_service_server::TrustRiskAssessmentServiceServer,
        trust_risk_signal_service_server::TrustRiskSignalServiceServer,
    },
};
use sqlx::PgPool;
use tokio::net::TcpListener;

use crate::{
    app::TrustRiskState,
    assessment_grpc::AssessmentService,
    config::{ProducerPolicy, RetentionConfig, TrustRiskConfig},
    ingress_grpc::SignalService,
};

const VALID_TOKEN: &str = "development-trust-risk-token-32-value";

fn test_config() -> TrustRiskConfig {
    let mut producers = BTreeMap::new();
    producers.insert(
        "billing-checkout-fixture".to_string(),
        ProducerPolicy {
            producer: "billing-checkout-fixture".to_string(),
            token: VALID_TOKEN.to_string(),
            signal_prefixes: vec![
                "network.".to_string(),
                "payment.".to_string(),
                "velocity.".to_string(),
            ],
            can_assess: true,
            can_label: true,
        },
    );

    TrustRiskConfig {
        environment: "test".to_string(),
        sentry_dsn: None,
        sentry_traces_sample_rate: 0.0,
        otlp_endpoint: None,
        otlp_authorization_header: None,
        database_url: "postgres://postgres:postgres@localhost:5432/test".to_string(),
        bind_addr: "127.0.0.1:0".parse().expect("valid address"),
        producers,
        operators: BTreeMap::new(),
        metrics_token: VALID_TOKEN.to_string(),
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

async fn spawn_provider(pool: PgPool) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind provider listener");
    let address = listener.local_addr().expect("provider address");
    let state = TrustRiskState::new(test_config(), pool);

    let signals = TrustRiskSignalServiceServer::new(SignalService::new(state.clone()));
    let assessments = TrustRiskAssessmentServiceServer::new(AssessmentService::new(state));

    let app = tonic::service::Routes::new(signals)
        .add_service(assessments)
        .into_axum_router();

    let server = tokio::spawn(async move {
        axum::serve(listener, app).await.expect("serve provider");
    });
    (format!("http://{address}"), server)
}

fn client_config(endpoint: String, token: &str) -> TrustRiskClientConfig {
    TrustRiskClientConfig::from_values("test", endpoint, token.to_string(), Duration::from_secs(5))
        .expect("valid client configuration")
}

fn sample_signal(id: &str) -> pb::RiskSignal {
    let signal_id = uuid::Uuid::new_v4().to_string();
    pb::RiskSignal {
        signal_id,
        schema_version: 1,
        producer: "billing-checkout-fixture".to_string(),
        signal_kind: "network.reputation".to_string(),
        occurred_at: Some(prost_types::Timestamp {
            seconds: 1_787_590_000,
            nanos: 0,
        }),
        partition_key: format!("regional:eu-west:principal:{id}"),
        subjects: vec![pb::SubjectReference {
            kind: pb::SubjectKind::Network.into(),
            namespace: "nvbes.network".to_string(),
            opaque_id: "network:018f7f2d-fc7d-7b7a".to_string(),
            scope: pb::DataScope::Regional.into(),
            tenant_id: None,
        }],
        attributes: HashMap::from([(
            "risk_score".to_string(),
            pb::AttributeValue {
                value: Some(pb::attribute_value::Value::UnsignedValue(85)),
            },
        )]),
        context: None,
        scope: pb::DataScope::Regional.into(),
    }
}

fn sample_assessment_request(key: &str) -> pb::AssessRiskRequest {
    let subject = pb::SubjectReference {
        kind: pb::SubjectKind::Principal.into(),
        namespace: "nvbes.identity".to_string(),
        opaque_id: "principal:018f7f2d-fc7d".to_string(),
        scope: pb::DataScope::Regional.into(),
        tenant_id: None,
    };
    pb::AssessRiskRequest {
        context: None,
        producer: "billing-checkout-fixture".to_string(),
        assessment_key: format!("checkout:{key}"),
        operation_class: "payment.checkout".to_string(),
        subjects: vec![subject],
        instantaneous_signals: vec![sample_signal("sig01")],
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn consumer_contract_submits_signals_and_assesses_risk(pool: PgPool) {
    let (url, server) = spawn_provider(pool).await;
    let client = TrustRiskClient::connect(client_config(url, VALID_TOKEN))
        .await
        .expect("consumer connects to provider");

    // 1. Submit Signals via gRPC wire
    let submit_req = pb::SubmitSignalsRequest {
        signals: vec![sample_signal("sig02")],
    };
    let submit_res = client
        .submit_signals(submit_req)
        .await
        .expect("signals submitted successfully");

    assert_eq!(submit_res.receipts.len(), 1);
    assert!(!submit_res.receipts[0].duplicate);

    // 2. Assess Risk via gRPC wire
    let assess_req = sample_assessment_request("eval-first");
    let evaluation = client
        .assess_risk(assess_req.clone())
        .await
        .expect("assessment succeeds");

    assert!(!evaluation.evaluation_id.is_empty());
    assert!(!evaluation.duplicate);
    assert_eq!(evaluation.rule_set_version, "baseline-v1");

    // 3. Idempotent replay on identical key
    let replay = client
        .assess_risk(assess_req)
        .await
        .expect("idempotent replay succeeds");

    assert_eq!(replay.evaluation_id, evaluation.evaluation_id);
    assert!(replay.duplicate);

    server.abort();
}

#[sqlx::test(migrations = "./migrations")]
async fn consumer_contract_maps_conflicts_and_unauthorized_errors(pool: PgPool) {
    let (url, server) = spawn_provider(pool).await;
    let client = TrustRiskClient::connect(client_config(url.clone(), VALID_TOKEN))
        .await
        .expect("consumer connects with valid token");

    let req = sample_assessment_request("conflict-key");
    client
        .assess_risk(req.clone())
        .await
        .expect("initial assessment succeeds");

    // Idempotency conflict: same assessment_key, altered operation_class
    let mut conflicting = req;
    conflicting.operation_class = "identity.login".to_string();
    let conflict_err = client
        .assess_risk(conflicting)
        .await
        .expect_err("conflicting assessment must fail");

    assert!(matches!(conflict_err, TrustRiskClientError::Conflict));

    // Authentication failure: caller token invalid
    let unauth_token = "invalid-token-with-at-least-32-chars-long";
    let unauth_client = TrustRiskClient::connect(client_config(url, unauth_token))
        .await
        .expect("client connects");

    let auth_err = unauth_client
        .assess_risk(sample_assessment_request("unauth-key"))
        .await
        .expect_err("unauthorized assessment must fail");

    assert!(matches!(auth_err, TrustRiskClientError::Unauthorized));

    server.abort();
}
