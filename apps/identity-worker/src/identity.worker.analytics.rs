use serde_json::Value;
use uuid::Uuid;

use crate::app::AppState;
use nvbes_product_analytics::ProductAnalyticsEvent;

pub(crate) fn capture_billing_webhook_analytics(
    state: &AppState,
    workspace_id: Uuid,
    event: &nvbes_billing::StripeWebhookEvent,
    plan_code: Option<&str>,
) {
    match event.event_type.as_str() {
        "customer.subscription.created" | "customer.subscription.updated" => {
            let Some(status) = subscription_status(&event.data_object) else {
                return;
            };
            if !matches!(status, "active" | "trialing") {
                return;
            }
            let mut analytics_event =
                ProductAnalyticsEvent::workspace("billing.subscription_activated", workspace_id)
                    .property("status", status);
            if let Some(plan_code) = plan_code {
                analytics_event = analytics_event.property("plan_code", plan_code.to_string());
            }
            state.product_analytics.capture(analytics_event);
        }
        "invoice.payment_failed" => {
            state.product_analytics.capture(
                ProductAnalyticsEvent::workspace("billing.payment_failed", workspace_id)
                    .property("status", "past_due"),
            );
        }
        _ => {}
    }
}

fn subscription_status(object: &Value) -> Option<&'static str> {
    object
        .get("status")
        .and_then(Value::as_str)
        .map(|status| nvbes_billing::stripe_subscription_status(Some(status)))
}
