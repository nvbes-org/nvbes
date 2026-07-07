const runtimeImplementedBillingRpcs = [
	{ proto: "GetBillingOverview", method: "get_billing_overview", evidence: "fetch_workspace_billing_overview" },
	{ proto: "GetBillingPortal", method: "get_billing_portal", evidence: "fetch_portal_view" },
	{ proto: "CreateCheckout", method: "create_checkout", evidence: "create_billing_checkout_session" },
	{ proto: "IngestUsage", method: "ingest_usage", evidence: "ingest_usage_event" },
	{ proto: "RunReconciliation", method: "run_reconciliation", evidence: "run_ledger_reconciliation" },
];

const futureDeclaredBillingRpcs = [
	{ proto: "CreatePortal", method: "create_portal" },
	{ proto: "GetEntitlements", method: "get_entitlements" },
	{ proto: "ListInvoices", method: "list_invoices" },
	{ proto: "GetInvoice", method: "get_invoice" },
	{ proto: "RecordLedgerEntry", method: "record_ledger_entry" },
	{ proto: "ListLedgerEntries", method: "list_ledger_entries" },
	{ proto: "IngestProviderWebhook", method: "ingest_provider_webhook" },
	{ proto: "ListProviderWebhookEvents", method: "list_provider_webhook_events" },
	{ proto: "ReconcileProviderWebhook", method: "reconcile_provider_webhook" },
	{ proto: "GetAdminCommandCenterBillingMetrics", method: "get_admin_command_center_billing_metrics" },
	{ proto: "GetAdminOperationsCenter", method: "get_admin_operations_center" },
	{ proto: "RunAdminOperationsAction", method: "run_admin_operations_action" },
	{ proto: "RunAdminBillingPlatformAction", method: "run_admin_billing_platform_action" },
	{ proto: "RunAdminBillingAction", method: "run_admin_billing_action" },
	{ proto: "RunAdminRevenueAction", method: "run_admin_revenue_action" },
];

export function checkBillingRpcRuntimeScope({ errors, gatewaySource, proto, protoPath, service }) {
	if (!service.includes("impl BillingService for BillingGrpcService")) {
		errors.push("apps/billing-service/src/billing.grpc.service.rs: missing impl BillingService for BillingGrpcService");
	}

	for (const rpc of runtimeImplementedBillingRpcs) {
		if (!proto.includes(`rpc ${rpc.proto}(`)) {
			errors.push(`${protoPath}: missing runtime Billing RPC ${rpc.proto}`);
		}
		if (!service.includes(`async fn ${rpc.method}`) || !service.includes(rpc.evidence)) {
			errors.push(`apps/billing-service/src/billing.grpc.service.rs: missing runtime Billing RPC ${rpc.method}`);
		}
	}

	for (const rpc of futureDeclaredBillingRpcs) {
		if (!proto.includes(`rpc ${rpc.proto}(`)) {
			errors.push(`${protoPath}: missing future-declared Billing RPC ${rpc.proto}`);
		}
		if (gatewaySource.includes(`.${rpc.method}(`)) {
			errors.push(`apps/gateway-cloud: must not call future-declared Billing RPC ${rpc.method} before runtime implementation exists`);
		}
	}
}
