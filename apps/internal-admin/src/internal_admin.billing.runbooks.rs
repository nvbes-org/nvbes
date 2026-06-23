use axum::{Json, Router, routing::get};
use serde::Serialize;

use crate::app::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BillingRunbook {
    PspOutage,
    WebhookLag,
    DuplicatePayment,
    TaxConfigError,
    LedgerImbalance,
    FailedExport,
}

#[derive(Debug, Serialize)]
pub struct BillingRunbookView {
    pub slug: &'static str,
    pub title: &'static str,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/admin/billing/runbooks", get(list_billing_runbooks_route))
}

async fn list_billing_runbooks_route() -> Json<Vec<BillingRunbookView>> {
    Json(list_billing_runbooks())
}

fn list_billing_runbooks() -> Vec<BillingRunbookView> {
    vec![
        runbook_view(BillingRunbook::PspOutage, "PSP outage"),
        runbook_view(BillingRunbook::WebhookLag, "Webhook lag"),
        runbook_view(BillingRunbook::DuplicatePayment, "Duplicate payment"),
        runbook_view(BillingRunbook::TaxConfigError, "Tax config error"),
        runbook_view(BillingRunbook::LedgerImbalance, "Ledger imbalance"),
        runbook_view(BillingRunbook::FailedExport, "Failed export"),
    ]
}

fn runbook_view(runbook: BillingRunbook, title: &'static str) -> BillingRunbookView {
    BillingRunbookView {
        slug: runbook_slug(runbook),
        title,
    }
}

fn runbook_slug(runbook: BillingRunbook) -> &'static str {
    match runbook {
        BillingRunbook::PspOutage => "psp-outage",
        BillingRunbook::WebhookLag => "webhook-lag",
        BillingRunbook::DuplicatePayment => "duplicate-payment",
        BillingRunbook::TaxConfigError => "tax-config-error",
        BillingRunbook::LedgerImbalance => "ledger-imbalance",
        BillingRunbook::FailedExport => "failed-export",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runbook_slugs_cover_critical_incidents() {
        assert_eq!(runbook_slug(BillingRunbook::PspOutage), "psp-outage");
        assert_eq!(
            runbook_slug(BillingRunbook::LedgerImbalance),
            "ledger-imbalance"
        );
    }
}
