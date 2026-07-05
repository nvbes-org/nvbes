import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const skippedDirs = new Set([".git", ".nx", "coverage", "dist", "node_modules", "target"]);
const textExtensions = new Set([".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".sql"]);

export function checkIdentityBillingRuntimeBoundary(errors) {
	checkIdentityRuntimeDoesNotOwnBilling(errors);
	checkBillingRuntimeOwnsWebhookPipeline(errors);
}

function walk(dir, results = []) {
	if (!existsSync(dir)) return results;
	for (const entry of readdirSync(dir)) {
		const path = join(dir, entry);
		const stat = statSync(path);
		if (stat.isDirectory()) {
			if (!skippedDirs.has(entry)) walk(path, results);
			continue;
		}
		const relativePath = relative(process.cwd(), path).replaceAll("\\", "/");
		const dot = relativePath.lastIndexOf(".");
		if (dot >= 0 && textExtensions.has(relativePath.slice(dot))) results.push(relativePath);
	}
	return results;
}

function checkIdentityRuntimeDoesNotOwnBilling(errors) {
	for (const file of [...walk("apps/account-service/src"), ...walk("apps/account-worker/src")]) {
		const content = readFileSync(file, "utf8");
		if (file.includes("identity.domains.billing")) {
			errors.push(`${file}: Account Service must not contain Billing domain runtime files`);
		}
		if (content.includes("nvbes_billing")) {
			errors.push(`${file}: Account Service must not import nvbes_billing; use Billing API boundaries instead`);
		}
		if (
			file.endsWith("identity.domains.mod.rs") &&
			(content.includes('identity.domains.billing.mod.rs') || /\bpub\s+mod\s+billing\b/.test(content))
		) {
			errors.push(`${file}: Account Service must not compile crate::domains::billing`);
		}
		for (const pattern of [
			/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+billing_[a-z0-9_]+\b/i,
			/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+invoice_estimates\b/i,
			/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+stripe_price_mappings\b/i,
			/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+subscriptions\b/i,
			/\bEnterpriseBilling[A-Za-z0-9_]*\b/,
			/["']\/enterprise\/billing["']/,
			/\bbilling\.(?:stripe|mollie)\.webhook\.process\b/,
			/\bbilling\.email\.send\b/,
			/\bJOB_(?:STRIPE|MOLLIE)_WEBHOOK_PROCESS\b/,
			/["']\/webhooks\/(?:stripe|mollie)["']/,
			/\bbilling_(?:checkout_started|portal_opened|operation_blocked)\b/,
			/\b(?:security-billing|billing-abuse|billing-guard)\b/,
		]) {
			if (pattern.test(content)) errors.push(`${file}: Identity runtime must not own Billing surface ${pattern}`);
		}
	}
	for (const manifest of ["apps/account-service/Cargo.toml", "apps/account-worker/Cargo.toml"]) {
		if (existsSync(manifest) && readFileSync(manifest, "utf8").includes("nvbes-billing")) {
			errors.push(`${manifest}: Identity runtime must not depend on nvbes-billing`);
		}
	}
	checkIdentityBetaToolsDoNotOwnBilling(errors);
	checkIdentityOpenapiAndClients(errors);
}

function checkIdentityBetaToolsDoNotOwnBilling(errors) {
	for (const file of walk("apps/account-service/src").filter((path) => path.includes("identity.tools.beta"))) {
		const content = readFileSync(file, "utf8");
		for (const pattern of [
			/\b(?:stripe|mollie|safe'?r|updat'?r|fast'?r)\b/i,
			/\b(?:checkout|portal|invoice|card|subscription|entitlement|usage)\b/i,
			/\b(?:webhook|dunning|reconciliation)\b/i,
			/\bNVBES_BILLING_[A-Z0-9_]+\b/,
			/\bbilling\.[a-z0-9_.-]+\b/i,
			/\bbilling_[a-z0-9_]+\b/i,
			/\b(?:provider_customer_id|provider_payment_method_id|provider_subscription_id|provider_invoice_id)\b/i,
			/\b(?:stripe_customer_id|stripe_subscription_id|mollie_customer_id|mollie_mandate_id)\b/i,
			/\/workspaces\/[^"']*\/billing(?:\/|\{|["'])/i,
		]) {
			if (pattern.test(content)) errors.push(`${file}: Identity beta tools must stay Identity-only and must not own Billing runtime ${pattern}`);
		}
	}
}

function checkIdentityOpenapiAndClients(errors) {
	const identityOpenapi = "apps/account-service/openapi.json";
	if (existsSync(identityOpenapi)) {
		const content = readFileSync(identityOpenapi, "utf8");
		if (/["']\/[^"']*\/billing(?:\/|\{|["'])/.test(content)) errors.push(`${identityOpenapi}: Identity OpenAPI must not expose Billing routes`);
		if (/"name"\s*:\s*"billing"/i.test(content) || /"tags"\s*:\s*\[[^\]]*"billing"/i.test(content)) errors.push(`${identityOpenapi}: Identity OpenAPI must not expose Billing tags`);
		for (const schema of [
			"BillingAccountView",
			"BillingPortalCapabilities",
			"BillingPortalCreditView",
			"BillingPortalInvoiceView",
			"BillingPortalView",
			"BillingWebhookResponse",
			"CheckoutSessionResponse",
			"EnterpriseBillingPlan",
			"EnterpriseBillingResponse",
			"EnterpriseInvoice",
			"PortalSessionResponse",
		]) {
			if (content.includes(`"${schema}"`)) errors.push(`${identityOpenapi}: Identity OpenAPI must not expose Billing schema ${schema}`);
		}
	}
	const workspaceBillingRouteFragments = [
		"/workspaces/{workspaceId}/billing/overview",
		"/workspaces/{workspaceId}/billing/usage",
		"/workspaces/{workspaceId}/billing/entitlements",
		"/workspaces/{workspaceId}/billing/checkout",
		"/workspaces/{workspaceId}/billing/portal",
		"/workspaces/{workspaceId}/billing/portal/view",
		"/workspaces/{workspaceId}/billing/invoices",
		"/workspaces/{workspaceId}/billing/cards",
		"/workspaces/{workspaceId}/billing/subscriptions",
		"/workspaces/{workspaceId}/billing/portal/invoices/{invoiceId}/pdf",
	];
	for (const file of [
		...walk("apps/account-service/src"),
		"apps/account-service/openapi.json",
		...walk("libs/ts/identity-client/src"),
		...walk("libs/ts/identity-sdk-core/src"),
	]) {
		if (!existsSync(file)) continue;
		const content = readFileSync(file, "utf8");
		for (const route of workspaceBillingRouteFragments) {
			if (content.includes(route)) errors.push(`${file}: Identity surfaces must not expose workspace Billing route ${route}`);
		}
		if (/\/workspaces\/\$\{[^}]+\}\/billing\//.test(content)) errors.push(`${file}: Identity clients must not construct workspace Billing route templates`);
		if (/\b(?:get|create|list|download)Billing(?:Overview|Usage|Entitlements|Checkout|Portal|Invoices|Cards|Subscriptions|InvoicePdf)\b/.test(content)) {
			errors.push(`${file}: Identity clients must not expose workspace Billing transport methods`);
		}
	}
}

function checkBillingRuntimeOwnsWebhookPipeline(errors) {
	checkRequiredEvidence(errors, "apps/billing-service/src/billing.domains.public_workspace.rs", [
		["/workspaces/{workspaceId}/billing/overview", "overview route"],
		["/workspaces/{workspaceId}/billing/usage", "usage route"],
		["/workspaces/{workspaceId}/billing/entitlements", "entitlements route"],
		["/workspaces/{workspaceId}/billing/checkout", "checkout route"],
		["/workspaces/{workspaceId}/billing/portal", "portal route"],
		["/workspaces/{workspaceId}/billing/invoices", "invoices route"],
		["/workspaces/{workspaceId}/billing/cards", "cards route"],
		["/workspaces/{workspaceId}/billing/subscriptions", "subscriptions route"],
	]);
	checkRequiredEvidence(errors, "apps/billing-service/src/billing.domains.webhooks.rs", [
		["/webhooks/stripe", "Stripe webhook route"],
		["/webhooks/mollie", "Mollie webhook route"],
		["body: Bytes", "raw webhook body extraction"],
		["stripe-signature", "Stripe signature header extraction"],
		["handle_stripe_webhook_intake", "Stripe intake enqueue"],
		["enqueue_mollie_webhook_job", "Mollie processing enqueue"],
		["record_billing_webhook", "Billing webhook metrics"],
	]);
	checkRequiredEvidence(errors, "libs/rust/billing/src/stripe_webhook_intake.rs", [
		["verify_stripe_signature", "Stripe signature verification"],
		["classify_webhook_retry", "Stripe webhook idempotency"],
		["enqueue_stripe_webhook_job", "Stripe async processing"],
	]);
	checkRequiredEvidence(errors, "apps/billing-worker/src/billing.worker.jobs.rs", [
		["JOB_STRIPE_WEBHOOK_PROCESS", "Stripe job"],
		["JOB_MOLLIE_WEBHOOK_PROCESS", "Mollie job"],
		["JOB_BILLING_EMAIL_SEND", "Billing email job"],
	]);
	checkRequiredEvidence(errors, "apps/billing-worker/src/billing.worker.jobs.stripe.rs", [
		["process_stripe_event", "Stripe processing"],
		["publish_workspace_billing_updates", "Stripe workspace update"],
		["capture_billing_webhook_analytics", "Stripe analytics"],
		["enqueue_billing_email_for_stripe_event", "Stripe email"],
	]);
	checkRequiredEvidence(errors, "apps/billing-worker/src/billing.worker.jobs.mollie.rs", [
		["process_mollie_payment_update_tx", "Mollie processing"],
		["publish_workspace_billing_updates", "Mollie workspace update"],
		["capture_mollie_webhook_analytics", "Mollie analytics"],
		["enqueue_billing_email_for_provider_payment", "Mollie email"],
	]);
	checkRequiredEvidence(errors, "apps/billing-worker/src/billing.worker.workspace_updates.rs", [
		["publish_workspace_updated", "workspace updated pubsub"],
		["publish_workspace_plan_updated", "workspace plan pubsub"],
	]);
	checkRequiredEvidence(errors, "apps/billing-worker/src/billing.worker.loop.rs", [
		["process_due_dunning_attempts", "dunning scheduler"],
		["record_billing_operation(\"dunning\"", "dunning observability"],
	]);
}

function checkRequiredEvidence(errors, file, expectations) {
	if (!existsSync(file)) {
		errors.push(`${file}: missing required Billing boundary evidence`);
		return;
	}
	const content = readFileSync(file, "utf8");
	for (const [needle, description] of expectations) {
		if (!content.includes(needle)) errors.push(`${file}: missing ${description}`);
	}
}
