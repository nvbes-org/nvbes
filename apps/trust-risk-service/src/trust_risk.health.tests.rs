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

#[cfg(feature = "database-tests")]
mod database {
    use std::time::Instant;

    use axum::body::Body;
    use tower::ServiceExt;

    use super::super::{ready, router};
    use crate::grpc_test_support;

    #[sqlx::test(migrations = "./migrations")]
    async fn readiness_reports_ready_when_database_and_projection_are_current(pool: sqlx::PgPool) {
        let state = grpc_test_support::state(pool);
        *state.projection_heartbeat.write().await = Some(Instant::now());
        let (status, body) = ready(axum::extract::State(state)).await;
        assert_eq!(status, axum::http::StatusCode::OK);
        assert_eq!(body.0.status, "ready");
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn readiness_route_returns_service_unavailable_without_heartbeat(pool: sqlx::PgPool) {
        let state = grpc_test_support::state(pool);
        let app = router(state);
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/health/ready")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        );
    }
}
