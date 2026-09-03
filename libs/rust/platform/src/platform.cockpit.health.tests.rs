use super::*;

#[test]
fn evaluates_healthy_runtime() {
    let aggregator = HealthAggregator::default();
    let probe = HealthProbeResult {
        service_id: ServiceId::Identity,
        live: true,
        ready: true,
        latency_ms: 35,
        message: None,
    };
    let health = aggregator.evaluate_runtime(probe);

    assert_eq!(health.status, HealthStatus::Healthy);
    assert_eq!(health.service_id, ServiceId::Identity);
    assert!(!health.is_cold_start);
}

#[test]
fn detects_cold_start_as_degraded_without_masking_origin() {
    let aggregator = HealthAggregator::new(500);
    let probe = HealthProbeResult {
        service_id: ServiceId::Account,
        live: true,
        ready: true,
        latency_ms: 850,
        message: None,
    };
    let health = aggregator.evaluate_runtime(probe);

    assert_eq!(health.status, HealthStatus::Degraded);
    assert!(health.is_cold_start);
    assert_eq!(health.service_id, ServiceId::Account);
    assert!(health.details.contains("Cold start detected"));
}

#[test]
fn handles_unavailable_and_not_ready_runtimes() {
    let aggregator = HealthAggregator::default();

    let dead_probe = HealthProbeResult {
        service_id: ServiceId::Email,
        live: false,
        ready: false,
        latency_ms: 12,
        message: Some("Connection refused".to_string()),
    };
    let dead_health = aggregator.evaluate_runtime(dead_probe);
    assert_eq!(dead_health.status, HealthStatus::Unavailable);
    assert_eq!(dead_health.details, "Connection refused");

    let not_ready_probe = HealthProbeResult {
        service_id: ServiceId::Billing,
        live: true,
        ready: false,
        latency_ms: 25,
        message: None,
    };
    let not_ready_health = aggregator.evaluate_runtime(not_ready_probe);
    assert_eq!(not_ready_health.status, HealthStatus::Degraded);
}

#[test]
fn aggregates_multiple_runtimes_correctly() {
    let aggregator = HealthAggregator::default();
    let probes = vec![
        HealthProbeResult {
            service_id: ServiceId::Identity,
            live: true,
            ready: true,
            latency_ms: 30,
            message: None,
        },
        HealthProbeResult {
            service_id: ServiceId::Billing,
            live: true,
            ready: false,
            latency_ms: 40,
            message: Some("Stripe test mode waiting webhook secret".to_string()),
        },
    ];

    let aggregate = aggregator.aggregate(probes);
    assert_eq!(aggregate.overall_status, HealthStatus::Degraded);
    assert_eq!(aggregate.degraded_count, 1);
    assert_eq!(aggregate.runtimes.len(), 2);
}
