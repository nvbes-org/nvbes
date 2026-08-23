use axum::{Router, body::Body, http::Request, http::StatusCode};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn simulate_routing_rule_requires_billing_platform_permission() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://localhost/internal_admin_billing_platform_simulation_test")
        .expect("lazy pool should build");
    let app = Router::new()
        .merge(crate::billing_platform_center_actions::router())
        .with_state(crate::app::AppState::new(
            nvbes_core::config::AppConfig::default(),
            pool,
        ));
    let workspace_id = Uuid::new_v4();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/workspaces/{workspace_id}/admin/billing-platform/routing-rules/simulate?country=FR&currency=EUR&payment_method=card&customer_type=b2b&amount_minor=2900"
                ))
                .header("x-nvbes-actor-principal-id", Uuid::new_v4().to_string())
                .header("x-nvbes-backoffice-role", "viewer")
                .body(Body::empty())
                .expect("request should build"),
        )
        .await
        .expect("route should respond");

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}
