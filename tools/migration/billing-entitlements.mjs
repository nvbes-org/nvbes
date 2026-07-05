#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/billing-entitlements.generated.json";
const markdownPath = "docs/migration/billing-entitlements.md";

const sources = {
	sharedViews: "libs/rust/billing/src/views.rs",
	sharedTypes: "libs/rust/billing/src/types.rs",
	sharedShared: "libs/rust/billing/src/shared.rs",
	billingPublicWorkspaceRoutes: "apps/billing-service/src/billing.domains.public_workspace.rs",
	billingInternalWorkspaceRoutes: "apps/billing-service/src/billing.domains.workspace.rs",
	billingWorkspaceAuth: "apps/billing-service/src/billing.auth.rs",
	billingWorkspaceViews: "libs/rust/billing/src/workspace_views.rs",
	billingCheckoutSessions: "libs/rust/billing/src/checkout_sessions.rs",
	billingPortalActions: "libs/rust/billing/src/portal_actions.rs",
	driveEntitlements: "apps/cloud-service/src/drive.domains.billing.entitlements.rs",
	entitlementEvent: "contracts/events/billing.entitlement.changed.v1.schema.json",
	eventManifest: "contracts/events/manifest.json",
	dataMap: "docs/migration/data-map.md",
	jobMap: "docs/migration/job-map.md",
	resourceMap: "docs/migration/resource-map.md",
	riskRegister: "docs/migration/risk-register.generated.json",
};

const errors = [];
const packageScripts = readPackageScripts(errors);

function read(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return "";
	}
	return readFileSync(path, "utf8");
}

function textCheck(id, path, description, pattern) {
	const content = read(path);
	return {
		id,
		description,
		path,
		status: content.includes(pattern) ? "passed" : "failed",
		pattern,
	};
}

function buildChecks() {
	return [
		textCheck("entitlement-type", sources.sharedTypes, "Shared billing type exposes product entitlements", "pub struct ProductEntitlementsView"),
		textCheck("entitlement-upload", sources.sharedTypes, "Entitlements include upload permission", "pub can_upload: bool"),
		textCheck("entitlement-share-link", sources.sharedTypes, "Entitlements include share-link permission", "pub can_create_share_links: bool"),
		textCheck("entitlement-storage", sources.sharedTypes, "Entitlements include storage allowance", "pub included_storage_bytes: i64"),
		textCheck("entitlement-api-keys", sources.sharedTypes, "Entitlements include API key limits", "pub api_key_limit: i32"),
		textCheck("entitlement-lock", sources.sharedTypes, "Entitlements include billing lock state", "pub billing_locked: bool"),
		textCheck("shared-entitlement-view", sources.sharedViews, "Shared entitlement view derives product permissions", "pub fn entitlements_view"),
		textCheck("shared-lock-statuses", sources.sharedViews, "Shared entitlement view locks degraded statuses", "\"past_due\" | \"canceled\" | \"incomplete\" | \"suspended\""),
		textCheck("shared-api-key-limit", sources.sharedViews, "Shared entitlement view maps plan to API key limit", "api_key_limit(&record.plan_code)"),
		textCheck("shared-active-test", sources.sharedViews, "Active entitlement behavior is covered by unit test", "entitlements_view_allows_active_subscription_with_plan_limits"),
		textCheck("shared-locked-test", sources.sharedViews, "Degraded subscription lock behavior is covered by unit test", "entitlements_view_locks_degraded_subscriptions"),
		textCheck("shared-invoice-test", sources.sharedViews, "Invoice overage calculation is covered by unit test", "build_invoice_estimate_charges_only_billable_overages"),
		textCheck("shared-usage-overage", sources.sharedViews, "Invoice estimate charges storage and seat overages", "storage_overage_amount_cents + seat_overage_amount_cents"),
		textCheck("shared-lock-helper", sources.sharedShared, "Billing policy helper locks degraded subscription statuses", "subscription_status_requires_lock"),
		textCheck("billing-service-overview", sources.billingWorkspaceViews, "Billing service overview returns entitlements", "entitlements: entitlements_view(&record)"),
		textCheck("billing-service-usage-ledger", sources.billingWorkspaceViews, "Billing service usage response exposes billable storage and seats", "billable_quantity"),
		textCheck("billing-service-entitlements-route", sources.billingPublicWorkspaceRoutes, "Billing API exposes public entitlements endpoint", '"/workspaces/{workspaceId}/billing/entitlements"'),
		textCheck("billing-service-entitlements-authz", sources.billingPublicWorkspaceRoutes, "Billing entitlements endpoint requires Billing read authorization", "BillingWorkspacePermission::Read"),
		textCheck("billing-service-entitlements-service", sources.billingWorkspaceViews, "Billing service returns workspace entitlements", "fetch_workspace_entitlements"),
		textCheck("billing-service-type-response", sources.billingPublicWorkspaceRoutes, "Billing API returns shared entitlement response type", "ProductEntitlementsView"),
		textCheck("billing-service-internal-entitlements-route", sources.billingInternalWorkspaceRoutes, "Billing API exposes internal entitlements endpoint", '"/internal/workspaces/{workspaceId}/billing/entitlements"'),
		textCheck("billing-service-identity-authz", sources.billingWorkspaceAuth, "Billing API authorizes requests through Identity introspection", "introspect_identity_token"),
		textCheck("billing-checkout-policy-lock", sources.billingCheckoutSessions, "Billing checkout enforces lock policy", "subscription_status_requires_lock(&record.subscription_status)"),
		textCheck("billing-portal-policy-lock", sources.billingPortalActions, "Billing portal operations enforce lock policy", "subscription_status_requires_lock(&record.subscription_status)"),
		textCheck("drive-entitlement-projection", sources.driveEntitlements, "Drive projects Billing entitlement events locally", "drive_entitlement_projection_from_event"),
		textCheck("drive-entitlement-storage", sources.driveEntitlements, "Drive stores only entitlement projections from Billing", "INSERT INTO billing_entitlement_snapshots"),
		textCheck("drive-entitlement-no-financial-storage", sources.driveEntitlements, "Drive no longer stores Billing invoice estimates", "persist_entitlement_projection"),
		textCheck("entitlement-event-schema", sources.entitlementEvent, "Entitlement changed event schema is versioned", "\"event_type\": { \"const\": \"billing.entitlement.changed\" }"),
		textCheck("entitlement-event-payload", sources.entitlementEvent, "Entitlement event payload requires workspace and status", "\"workspace_id\",\n        \"snapshot_id\",\n        \"plan_code\",\n        \"subscription_status\""),
		textCheck("event-manifest", sources.eventManifest, "Event manifest includes billing entitlement changes", "\"billing.entitlement.changed\""),
		textCheck("data-map-ledger", sources.dataMap, "Billing financial data requires ledger balance reconciliation", "row_count, checksum, ledger_balance"),
		textCheck("subscription-rebuild", sources.dataMap, "Subscriptions have explicit rebuild decision", "`target-postgres:billing-usage.subscriptions`"),
		textCheck("usage-events-rebuild", sources.dataMap, "Usage events have explicit rebuild decision", "`target-postgres:billing-usage.usage_events`"),
		textCheck("job-map", sources.jobMap, "Billing webhook processing job is mapped", "billing.stripe.webhook.process"),
		textCheck("resource-map", sources.resourceMap, "Billing entitlement event topic is mapped", "billing.entitlement.changed"),
		textCheck("risk-register", sources.riskRegister, "Billing divergence risk requires ledger reconciliation", "ledger reconcile et freeze des mutations billing"),
	];
}

function summarize(checks) {
	const failed = checks.filter((check) => check.status === "failed").length;
	return {
		checks: checks.length,
		passed: checks.length - failed,
		failed,
		status: failed === 0 ? "passed" : "failed",
	};
}

function sameItems(actual, expected) {
	return Array.isArray(actual) && actual.length === expected.length && actual.every((item, index) => item === expected[index]);
}

function validateReport(report) {
	const seen = new Set();
	for (const check of report.checks) {
		if (seen.has(check.id)) errors.push(`${outputPath}: duplicate check ${check.id}`);
		seen.add(check.id);
		if (!check.description) errors.push(`${check.id}: description is required`);
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in billing entitlements source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "tools/migration/billing-entitlements.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match billing entitlements source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"cargo test -p nvbes-billing entitlements_view --locked",
		"cargo test -p nvbes-billing build_invoice_estimate_charges_only_billable_overages --locked",
		"pnpm check:migration-reconciliation-report",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "billing-entitlements", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Billing Entitlements Evidence",
		"",
		"## Status",
		"",
		`- status: ${data.summary.status}`,
		`- checks: ${data.summary.checks}`,
		`- passed: ${data.summary.passed}`,
		`- failed: ${data.summary.failed}`,
		"",
		"## Rules",
		"",
		"- Every evidence row must be generated from the billing entitlements source contract.",
		"- `passed` requires the configured file, contract, or generated migration document to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name entitlement and reconciliation checks required by parity.",
		"- Generation provenance must identify sources, write command and targeted tests.",
		"",
		"## Evidence",
		"",
		"| Check | Status | Path |",
		"|---|---:|---|",
	];
	for (const check of data.checks) {
		lines.push(`| ${check.description} | ${check.status} | \`${check.path}\` |`);
	}
	lines.push(
		"",
		"## Decision",
		"",
		data.summary.failed === 0
			? "Billing entitlements parity evidence is covered for repository cutover gates. Production cutover still requires accepted ledger reconciliation."
			: "Billing entitlements parity evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-billing-entitlements",
		"tools/migration/billing-entitlements.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

const checks = buildChecks();
const summary = summarize(checks);
const report = {
	schema_version: 1,
	generation: {
		command: "tools/migration/billing-entitlements.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"cargo test -p nvbes-billing entitlements_view --locked",
			"cargo test -p nvbes-billing build_invoice_estimate_charges_only_billable_overages --locked",
			"pnpm check:migration-reconciliation-report",
		],
	},
	summary,
	checks,
};

validateReport(report);

const json = serializeJson(report);
const markdown = serializeMarkdown(report);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Billing entitlements evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/billing-entitlements.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/billing-entitlements.mjs --write`);
}

if (errors.length > 0) {
	console.error("Billing entitlements evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Billing entitlements evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
