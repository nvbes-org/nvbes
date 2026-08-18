#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/billing-webhook-idempotency.generated.json";
const markdownPath = "docs/migration/billing-webhook-idempotency.md";

const sources = {
	identityOpenapi: "apps/identity-service/openapi.json",
	identityDomainsRouter: "apps/identity-service/src/identity.domains.mod.rs",
	billingServiceRoutes: "apps/billing-service/src/billing.domains.webhooks.rs",
	stripeIntake: "libs/rust/billing/src/stripe_webhook_intake.rs",
	billingJobs: "libs/rust/billing/src/jobs.rs",
	workerDispatcher: "apps/billing-worker/src/billing.worker.jobs.rs",
	stripeWorker: "apps/billing-worker/src/billing.worker.jobs.stripe.rs",
	mollieWorker: "apps/billing-worker/src/billing.worker.jobs.mollie.rs",
	workspaceUpdates: "apps/billing-worker/src/billing.worker.workspace_updates.rs",
	billingEmail: "apps/billing-worker/src/billing.worker.email.rs",
	billingEmailDelivery: "apps/billing-worker/src/billing.worker.email.delivery.rs",
	workerLoop: "apps/billing-worker/src/billing.worker.loop.rs",
	dunningDb: "libs/rust/billing/src/dunning_db.rs",
	dunningJobs: "libs/rust/billing/src/dunning_jobs.rs",
	providerEventsDb: "libs/rust/billing/src/db.provider_events.rs",
	migration: "apps/billing-service/migrations/0002_billing_platform_core.sql",
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

function absentTextCheck(id, path, description, pattern) {
	const content = read(path);
	return {
		id,
		description,
		path,
		status: content.includes(pattern) ? "failed" : "passed",
		pattern,
	};
}

function buildChecks() {
	return [
		absentTextCheck("identity-openapi-no-psp-webhook", sources.identityOpenapi, "Identity OpenAPI no longer exposes PSP webhook routes", "/webhooks/stripe"),
		absentTextCheck("identity-router-no-billing-merge", sources.identityDomainsRouter, "Identity router no longer merges legacy billing routes", ".merge(billing::routes::router(state))"),
		textCheck("billing-service-stripe-route", sources.billingServiceRoutes, "billing-service accepts Stripe webhooks", 'route("/webhooks/stripe", post(handle_stripe_webhook))'),
		textCheck("billing-service-mollie-route", sources.billingServiceRoutes, "billing-service accepts Mollie webhooks", 'route("/webhooks/mollie", post(handle_mollie_webhook))'),
		textCheck("billing-service-stripe-rate-limit", sources.billingServiceRoutes, "Stripe webhook intake is rate limited in billing-service", "enforce_stripe_webhook_rate_limit"),
		textCheck("billing-service-mollie-rate-limit", sources.billingServiceRoutes, "Mollie webhook intake is rate limited in billing-service", "enforce_mollie_webhook_rate_limit"),
		textCheck("stripe-intake-signature", sources.stripeIntake, "Stripe intake verifies signatures before enqueueing", "verify_stripe_signature(webhook_secret, signature_header, payload)"),
		textCheck("stripe-provider-event-lock", sources.stripeIntake, "Stripe intake locks existing provider event rows", "WHERE provider_event_id = $1\n        FOR UPDATE"),
		textCheck("stripe-duplicate-decision", sources.stripeIntake, "Processed Stripe webhooks are treated as duplicates", 'Some("processed") => WebhookRetryDecision::Duplicate'),
		textCheck("stripe-replay-decision", sources.stripeIntake, "Failed or received Stripe webhooks are replayable", 'Some("failed" | "received") => WebhookRetryDecision::ReplayFailed'),
		textCheck("stripe-duplicate-test", sources.stripeIntake, "Unit test covers processed duplicate classification", "classify_webhook_retry_treats_processed_as_duplicate"),
		textCheck("stripe-failed-replay-test", sources.stripeIntake, "Unit test covers failed replay classification", "classify_webhook_retry_treats_failed_as_replayable"),
		textCheck("stripe-received-replay-test", sources.stripeIntake, "Unit test covers received replay classification", "classify_webhook_retry_treats_received_as_replayable"),
		textCheck("stripe-replay-update", sources.stripeIntake, "Stripe replay path resets failed or received events", "WHEN billing_webhook_events.status IN ('failed', 'received') THEN 'received'::billing_webhook_status"),
		textCheck("stripe-enqueue", sources.stripeIntake, "Stripe intake enqueues async processing", "enqueue_stripe_webhook_job(redis, payload_json, &event.id)"),
		textCheck("stripe-job-type", sources.billingJobs, "Stripe webhook processing has a dedicated queue", 'JOB_STRIPE_WEBHOOK_PROCESS: &str = "billing.stripe.webhook.process"'),
		textCheck("mollie-job-type", sources.billingJobs, "Mollie webhook processing has a dedicated queue", 'JOB_MOLLIE_WEBHOOK_PROCESS: &str = "billing.mollie.webhook.process"'),
		textCheck("billing-email-job-type", sources.billingJobs, "Billing email submission has a dedicated integration queue", 'JOB_BILLING_EMAIL_SUBMIT: &str = "billing.integration.email.submit"'),
		textCheck("queue-idempotency-key", sources.billingJobs, "Queued webhook jobs use provider_event_id as idempotency key", "idempotency_key: Some(provider_event_id.to_string())"),
		textCheck("worker-claims-runtime-queues", sources.workerDispatcher, "Billing worker claims Stripe, Mollie, and billing email queues", "const BILLING_QUEUES: [&str; 3]"),
		textCheck("worker-dispatches-stripe", sources.workerDispatcher, "Billing worker dispatches Stripe webhook jobs", "stripe::process_stripe_webhook_job(state, job).await"),
		textCheck("worker-dispatches-mollie", sources.workerDispatcher, "Billing worker dispatches Mollie webhook jobs", "mollie::process_mollie_webhook_job(state, job).await"),
		textCheck("worker-dispatches-billing-email", sources.workerDispatcher, "Billing worker dispatches billing email jobs", "process_billing_email_job(state, job).await"),
		textCheck("stripe-worker-processes-domain", sources.stripeWorker, "Stripe worker calls billing-domain processing", "nvbes_billing::stripe_webhook_processing::process_stripe_event"),
		textCheck("stripe-worker-publishes-workspace-update", sources.stripeWorker, "Stripe worker publishes Billing workspace updates", "publish_workspace_billing_updates(state, workspace_id).await"),
		textCheck("stripe-worker-emails", sources.stripeWorker, "Stripe worker enqueues billing emails after processing", "enqueue_billing_email_for_stripe_event"),
		textCheck("billing-email-domain-queue", sources.billingEmail, "Billing email enqueue uses the Billing integration queue", "let queue = nvbes_billing::jobs::JOB_BILLING_EMAIL_SUBMIT"),
		absentTextCheck("billing-email-no-identity-queue", sources.billingEmail, "Billing email enqueue does not use Identity email.send queue", '"email.send"'),
		textCheck("billing-email-local-delivery", sources.billingEmailDelivery, "Billing worker submits commands through the shared email service", "state.email.send(command).await"),
		textCheck("billing-email-domain-header", sources.billingEmail, "Billing email commands carry the Billing category", "category: EmailCategory::Billing"),
		textCheck("mollie-worker-processes-domain", sources.mollieWorker, "Mollie worker calls billing-domain processing", "process_mollie_payment_update_tx"),
		textCheck("mollie-worker-publishes-workspace-update", sources.mollieWorker, "Mollie worker publishes Billing workspace updates", "publish_workspace_billing_updates(state, workspace_id).await"),
		textCheck("mollie-worker-emails", sources.mollieWorker, "Mollie worker enqueues billing emails after processing", "enqueue_billing_email_for_provider_payment"),
		textCheck("mollie-worker-marks-failed", sources.mollieWorker, "Mollie worker records failed provider events", "mark_provider_event_failed"),
		textCheck("mollie-worker-marks-processed", sources.mollieWorker, "Mollie worker records processed provider events", "mark_provider_event_processed"),
		textCheck("billing-worker-pubsub-workspace-updated", sources.workspaceUpdates, "Billing worker publishes workspace:updated pubsub events", "publish_workspace_updated"),
		textCheck("billing-worker-pubsub-plan-updated", sources.workspaceUpdates, "Billing worker publishes workspace:plan_updated pubsub events", "publish_workspace_plan_updated"),
		textCheck("billing-email-provider-neutral-payment", sources.billingEmail, "Billing email enqueue accepts provider-neutral payment events", "enqueue_billing_email_for_provider_payment"),
		textCheck("stripe-webhook-records-dunning-failure", sources.dunningDb, "Billing domain opens dunning cases after provider payment failures", "record_payment_failure_tx"),
		textCheck("stripe-webhook-records-dunning-success", sources.dunningDb, "Billing domain closes dunning cases after provider payment success", "record_payment_success_tx"),
		textCheck("billing-worker-runs-dunning", sources.workerLoop, "Billing worker schedules dunning processing", "run_billing_dunning_if_due"),
		textCheck("billing-worker-dunning-observability", sources.workerLoop, "Billing worker records dunning processing metrics", 'record_billing_operation("dunning"'),
		textCheck("billing-dunning-job-skip-locked", sources.dunningJobs, "Dunning processing claims due attempts safely", "FOR UPDATE OF attempt SKIP LOCKED"),
		textCheck("billing-dunning-job-validates-batch-size", sources.dunningJobs, "Dunning processing validates batch size", "DunningProcessingError::InvalidBatchSize"),
		textCheck("provider-event-upsert", sources.providerEventsDb, "Provider event intake is idempotent per provider event", "ON CONFLICT (provider, provider_event_id) DO UPDATE"),
		textCheck("provider-event-unique", sources.migration, "Database constrains provider event uniqueness per provider", "UNIQUE (provider, provider_event_id)"),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in billing webhook source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "node tools/migration/billing-webhook-idempotency.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match billing webhook source contract`);
	}
	if (report.generation?.targeted_test !== "cargo test -p nvbes-billing classify_webhook_retry --locked") {
		errors.push(`${outputPath}: generation.targeted_test is invalid`);
	}
	if (report.generation?.targeted_test) {
		errors.push(...validateProofCommand({ id: "billing-webhook-idempotency", proof: report.generation.targeted_test }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Billing Webhook Idempotency Evidence",
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
		"- Every evidence row must be generated from the billing webhook source contract.",
		"- `passed` requires the configured source to satisfy the expected presence or absence pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted test must name the webhook idempotency check required by parity.",
		"- Generation provenance must identify sources, write command and targeted test.",
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
			? "Billing webhook idempotency and replay evidence is covered for repository cutover gates."
			: "Billing webhook idempotency and replay evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-billing-webhook-idempotency",
		"node tools/migration/billing-webhook-idempotency.mjs --write",
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
		command: "node tools/migration/billing-webhook-idempotency.mjs --write",
		sources: Object.values(sources),
		targeted_test: "cargo test -p nvbes-billing classify_webhook_retry --locked",
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
	console.log(`Billing webhook idempotency evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/billing-webhook-idempotency.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/billing-webhook-idempotency.mjs --write`);
}

if (errors.length > 0) {
	console.error("Billing webhook idempotency evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Billing webhook idempotency evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
