use axum::{Json, Router, routing::get};
use serde_json::{Value, json};

use crate::app::AppState;
use crate::openapi_paths::paths;
use crate::openapi_schemas::components;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/openapi.json", get(openapi_route))
}

async fn openapi_route() -> Json<Value> {
    Json(openapi_document())
}

pub fn openapi_document() -> Value {
    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "nvbes Internal Admin API",
            "version": "0.1.0",
            "description": "Private back-office API for enterprise operations. All private routes require internal authentication and operator context headers.",
            "contact": {"name": "nvbes", "url": "https://nvbes.fr"},
            "license": {"name": "UNLICENSED"}
        },
        "servers": [
            {"url": "https://internal-admin.nvbes.fr", "description": "Production"},
            {"url": "http://localhost:8080", "description": "Development"}
        ],
        "tags": [
            {"name": "command", "description": "Enterprise command center"},
            {"name": "audit", "description": "Audit timeline and evidence"},
            {"name": "revenue", "description": "Revenue operations"},
            {"name": "billing-platform", "description": "Billing provider operations"},
            {"name": "developer", "description": "Developer platform operations"},
            {"name": "entitlements", "description": "Feature and quota entitlement operations"},
            {"name": "usage", "description": "Usage metering operations"},
            {"name": "communications", "description": "Email and webhook operations"},
            {"name": "compliance", "description": "GDPR, consent, and suppression operations"},
            {"name": "region", "description": "Data residency operations"},
            {"name": "risk", "description": "Risk decision operations"},
            {"name": "operations", "description": "Operational replay and incident workflows"},
            {"name": "governance", "description": "Customer and identity governance mutations"}
        ],
        "security": [{"internalToken": [], "operatorContext": []}],
        "paths": paths(),
        "components": components()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openapi_exposes_internal_admin_contract_entrypoints() {
        let document = openapi_document();
        assert_eq!(document["openapi"], "3.1.0");
        assert!(document["paths"]["/admin/command-center"]["get"].is_object());
        assert!(
            document["paths"]["/workspaces/{workspaceId}/admin/audit-events"]["get"].is_object()
        );
        assert!(
            document["paths"]["/workspaces/{workspaceId}/admin/revenue/invoices/{invoiceId}/hold"]
                ["post"]
                .is_object()
        );
    }

    #[test]
    fn critical_mutations_document_idempotency_and_dual_control_headers() {
        let document = openapi_document();
        let parameters =
            document["paths"]["/admin/users/{principalId}/suspend"]["post"]["parameters"]
                .as_array()
                .expect("parameters should be an array");
        let refs = parameters
            .iter()
            .filter_map(|parameter| parameter.get("$ref").and_then(Value::as_str))
            .collect::<Vec<_>>();

        assert!(refs.contains(&"#/components/parameters/IdempotencyKey"));
        assert!(refs.contains(&"#/components/parameters/BackofficeRole"));
        assert!(refs.contains(&"#/components/parameters/SecondApproverPrincipalId"));
        assert!(refs.contains(&"#/components/parameters/SecondApproverRole"));
    }

    #[test]
    fn action_result_contract_matches_backoffice_mutation_responses() {
        let document = openapi_document();
        let required = document["components"]["schemas"]["RevenueActionResult"]["required"]
            .as_array()
            .expect("required fields should be documented");
        let required_fields = required
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();

        assert!(required_fields.contains(&"object_id"));
        assert!(required_fields.contains(&"action_kind"));
        assert!(required_fields.contains(&"status"));
        assert!(required_fields.contains(&"audit_action"));
    }

    #[test]
    fn openapi_documents_actionable_center_mutations() {
        let document = openapi_document();
        for path in [
            "/workspaces/{workspaceId}/admin/billing-platform/kyc-profiles/{profileId}/reject",
            "/workspaces/{workspaceId}/admin/developer/clients/{clientId}/rotate-secret",
            "/workspaces/{workspaceId}/admin/entitlements/quota-overrides",
            "/workspaces/{workspaceId}/admin/usage/meters/freeze",
            "/workspaces/{workspaceId}/admin/communications/suppressions",
            "/workspaces/{workspaceId}/admin/compliance/principals/{principalId}/erasure-request",
            "/workspaces/{workspaceId}/admin/region/workspaces/{targetWorkspaceId}/exceptions",
            "/workspaces/{workspaceId}/admin/risk/signals/{signalId}/resolve",
            "/workspaces/{workspaceId}/admin/operations/export-runs/{exportRunId}/replay",
        ] {
            assert!(document["paths"][path]["post"].is_object(), "{path}");
        }
    }

    #[test]
    fn openapi_documents_billing_routing_simulation() {
        let document = openapi_document();
        let operation = &document["paths"]["/workspaces/{workspaceId}/admin/billing-platform/routing-rules/simulate"]
            ["get"];
        assert!(operation.is_object());
        assert_eq!(operation["operationId"], "simulateBillingRoutingRule");
        assert_eq!(
            operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/RoutingRuleSimulationResult"
        );

        let parameters = operation["parameters"]
            .as_array()
            .expect("parameters should be documented");
        let names = parameters
            .iter()
            .filter_map(|parameter| parameter.get("name").and_then(Value::as_str))
            .collect::<Vec<_>>();
        assert!(names.contains(&"workspaceId"));
        assert!(names.contains(&"currency"));
        assert!(names.contains(&"amount_minor"));
        assert!(document["components"]["schemas"]["MatchedRoutingRule"].is_object());
    }
}
