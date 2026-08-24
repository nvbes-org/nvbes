use std::time::{Duration, Instant};

use super::{heartbeat_current, live};

#[tokio::test]
async fn liveness_is_shallow() {
    let response = live().await;
    assert_eq!(response.0.status, "alive");
    assert_eq!(response.0.service, "nvbes-trust-risk-service");
}

#[test]
fn readiness_requires_a_current_projection_heartbeat() {
    let now = Instant::now();
    assert!(!heartbeat_current(None, now, Duration::from_secs(30)));
    assert!(heartbeat_current(
        Some(now - Duration::from_secs(29)),
        now,
        Duration::from_secs(30)
    ));
    assert!(!heartbeat_current(
        Some(now - Duration::from_secs(31)),
        now,
        Duration::from_secs(30)
    ));
}
