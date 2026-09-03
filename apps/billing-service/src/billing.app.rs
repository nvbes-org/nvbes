use std::sync::Arc;

use axum::{Router, routing::{get, post}};
use metrics_exporter_prometheus::PrometheusHandle;
use sqlx::PgPool;

use crate::{
    auth::TokenVerifier,
    checkout::create_checkout_handler,
    config::BillingConfig,
    health, metrics,
    plans::list_plans_handler,
    portal::{create_portal_handler, get_overview_handler},
    reconciliation::{
        operator_list_reconciliations_handler,
        operator_overview_handler,
        operator_resolve_reconciliation_handler,
    },
    webhooks::stripe_webhook_handler,
};

#[derive(Clone)]
pub struct BillingState {
    pub db: PgPool,
    pub config: BillingConfig,
    pub metrics: Arc<PrometheusHandle>,
    pub tokens: TokenVerifier,
}

pub fn create_router(state: BillingState) -> Router {
    Router::new()
        .merge(health::router())
        .merge(metrics::router())
        .route("/webhooks/stripe", post(stripe_webhook_handler))
        .route("/billing/plans", get(list_plans_handler))
        .route("/workspaces/{id}/billing/checkout", post(create_checkout_handler))
        .route("/workspaces/{id}/billing/portal", post(create_portal_handler))
        .route("/workspaces/{id}/billing/overview", get(get_overview_handler))
        .route("/operator/billing/overview", get(operator_overview_handler))
        .route("/operator/billing/reconciliations", get(operator_list_reconciliations_handler))
        .route("/operator/billing/reconciliations/{id}/resolve", post(operator_resolve_reconciliation_handler))
        .with_state(state)
}
