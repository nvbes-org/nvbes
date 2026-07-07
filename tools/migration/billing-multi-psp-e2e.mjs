#!/usr/bin/env node
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/billing-multi-psp-e2e.generated.json";
const markdownPath = "docs/migration/billing-multi-psp-e2e.md";

const sources = {
	billingServiceWebhooks: "apps/billing-service/src/billing.domains.webhooks.rs",
	stripeIntake: "libs/rust/billing/src/stripe_webhook_intake.rs",
	mollieWebhooks: "libs/rust/billing/src/mollie.webhooks.rs",
	jobs: "libs/rust/billing/src/jobs.rs",
	workerJobs: "apps/billing-worker/src/billing.worker.jobs.rs",
	stripeWorker: "apps/billing-worker/src/billing.worker.jobs.stripe.rs",
	mollieWorker: "apps/billing-worker/src/billing.worker.jobs.mollie.rs",
	workerEmail: "apps/billing-worker/src/billing.worker.email.rs",
	workerAnalytics: "apps/billing-worker/src/billing.worker.analytics.rs",
	workerWorkspaceUpdates: "apps/billing-worker/src/billing.worker.workspace_updates.rs",
	workerLoop: "apps/billing-worker/src/billing.worker.loop.rs",
	workspaceEffects: "libs/rust/billing/src/stripe_webhook_workspace_effects.rs",
	continuityEvidence: "docs/migration/billing-multi-psp-continuity.generated.json",
};

const targetedTests = [
	"pnpm check:migration-billing-webhook-idempotency",
	"pnpm check:migration-billing-multi-psp-continuity",
	"cargo test -p nvbes-billing provider_subscription --locked",
	"cargo test -p nvbes-billing workspace_effects_apply_only_to_primary_provider_subscription --locked",
	"cargo test -p nvbes-billing provider_code --locked",
	"cargo test -p nvbes-billing cb_ --locked",
];

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

function jsonStatusCheck(id, path, description, expectedPath, expectedValue) {
	let actual;
	try {
		actual = expectedPath.reduce((value, field) => value?.[field], JSON.parse(read(path)));
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
	}
	return {
		id,
		description,
		path,
		status: actual === expectedValue ? "passed" : "failed",
		pattern: `${expectedPath.join(".")}=${expectedValue}`,
	};
}

function identityBoundaryCheck() {
	const forbidden = [
		"/webhooks/stripe",
		"/webhooks/mollie",
		"billing.stripe.webhook.process",
		"billing.mollie.webhook.process",
		"process_stripe_webhook_job",
		"process_mollie_webhook_job",
		"handle_stripe_webhook_intake",
	];
	const matches = [];
	for (const path of rustFiles("apps/account-service/src").concat(rustFiles("apps/account-worker/src"))) {
		const content = read(path);
		for (const pattern of forbidden) {
			if (content.includes(pattern)) matches.push(`${path}: ${pattern}`);
		}
	}
	return {
		id: "identity-no-psp-webhook-runtime",
		description: "Account Service and worker do not own PSP webhook runtime",
		path: "apps/account-service/src + apps/account-worker/src",
		status: matches.length === 0 ? "passed" : "failed",
		pattern: `forbidden:${forbidden.join(",")}`,
		matches,
	};
}

function rustFiles(root) {
	if (!existsSync(root)) return [];
	const entries = readdirSync(root).flatMap((entry) => {
		const path = join(root, entry);
		if (statSync(path).isDirectory()) return rustFiles(path);
		return path.endsWith(".rs") ? [path] : [];
	});
	return entries;
}

function buildChecks() {
	return [
		textCheck("billing-service-stripe-route", sources.billingServiceWebhooks, "billing-service exposes Stripe webhook intake", '"/webhooks/stripe"'),
		textCheck("billing-service-mollie-route", sources.billingServiceWebhooks, "billing-service exposes Mollie webhook intake", '"/webhooks/mollie"'),
		textCheck("stripe-signature-intake", sources.stripeIntake, "Stripe intake verifies raw webhook signature", "verify_stripe_signature"),
		textCheck("stripe-async-queue", sources.stripeIntake, "Stripe intake enqueues asynchronous processing", "enqueue_stripe_webhook_job"),
		textCheck("mollie-id-only-intake", sources.mollieWebhooks, "Mollie classic webhook intake parses id-only callback", "mollie_classic_webhook_is_id_only_and_fetch_required"),
		textCheck("mollie-async-queue", sources.billingServiceWebhooks, "Mollie intake enqueues asynchronous processing", "enqueue_mollie_webhook_job"),
		textCheck("queue-stripe", sources.jobs, "Stripe webhook queue name is provider-scoped", 'JOB_STRIPE_WEBHOOK_PROCESS: &str = "billing.stripe.webhook.process"'),
		textCheck("queue-mollie", sources.jobs, "Mollie webhook queue name is provider-scoped", 'JOB_MOLLIE_WEBHOOK_PROCESS: &str = "billing.mollie.webhook.process"'),
		textCheck("worker-stripe-dispatch", sources.workerJobs, "Billing worker dispatches Stripe webhook jobs", "stripe::process_stripe_webhook_job"),
		textCheck("worker-mollie-dispatch", sources.workerJobs, "Billing worker dispatches Mollie webhook jobs", "mollie::process_mollie_webhook_job"),
		textCheck("worker-email-dispatch", sources.workerJobs, "Billing worker dispatches billing email jobs", "JOB_BILLING_EMAIL_SEND"),
		textCheck("stripe-worker-processing", sources.stripeWorker, "Stripe worker processes Billing webhook events", "process_stripe_event"),
		textCheck("stripe-worker-analytics", sources.stripeWorker, "Stripe worker captures Billing analytics", "capture_billing_webhook_analytics"),
		textCheck("stripe-worker-email", sources.stripeWorker, "Stripe worker enqueues Billing email", "enqueue_billing_email_for_stripe_event"),
		textCheck("mollie-worker-processing", sources.mollieWorker, "Mollie worker processes provider payment updates", "process_mollie_payment_update_tx"),
		textCheck("mollie-worker-finalize", sources.mollieWorker, "Mollie worker finalizes initial subscriptions", "finalize_mollie_initial_subscription_tx"),
		textCheck("mollie-worker-email", sources.mollieWorker, "Mollie worker enqueues Billing email", "enqueue_billing_email_for_provider_payment"),
		textCheck("worker-email-queue", sources.workerEmail, "Billing email uses the Billing worker queue", "billing_email_uses_billing_worker_queue"),
		textCheck("worker-analytics-module", sources.workerAnalytics, "Billing worker owns Billing webhook analytics", "capture_billing_webhook_analytics"),
		textCheck("worker-workspace-update-pubsub", sources.workerWorkspaceUpdates, "Billing worker publishes workspace update events after PSP webhooks", "publish_workspace_updated"),
		textCheck("worker-workspace-plan-pubsub", sources.workerWorkspaceUpdates, "Billing worker publishes workspace plan update events after PSP webhooks", "publish_workspace_plan_updated"),
		textCheck("worker-workspace-plan-test", sources.workerWorkspaceUpdates, "Billing worker tests plan update payload stability", "workspace_plan_update_payload_preserves_plan_code"),
		textCheck("worker-dunning-loop", sources.workerLoop, "Billing worker owns dunning processing loop", "process_due_dunning_attempts"),
		textCheck("workspace-primary-only", sources.workspaceEffects, "Workspace effects apply only to primary provider subscriptions", "workspace_effects_apply_only_to_primary_provider_subscription"),
		textCheck("workspace-dunning-effect", sources.workspaceEffects, "Workspace effects update dunning state from webhook outcomes", "record_payment_failure_tx"),
		jsonStatusCheck("continuity-evidence-passed", sources.continuityEvidence, "Multi-PSP continuity evidence is generated and passed", ["summary", "status"], "passed"),
		identityBoundaryCheck(),
	];
}

function summarize(checks) {
	const failed = checks.filter((check) => check.status === "failed").length;
	return { checks: checks.length, passed: checks.length - failed, failed, status: failed === 0 ? "passed" : "failed" };
}

function validateReport(report) {
	const seen = new Set();
	for (const check of report.checks) {
		if (seen.has(check.id)) errors.push(`${outputPath}: duplicate check ${check.id}`);
		seen.add(check.id);
		if (!check.description) errors.push(`${check.id}: description is required`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
		if (check.matches?.length) errors.push(`${check.id}: ${check.matches.join("; ")}`);
	}
	for (const [field, value] of Object.entries(summarize(report.checks))) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "node tools/migration/billing-multi-psp-e2e.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "billing-multi-psp-e2e", proof: command }, packageScripts));
	}
	if (!packageScripts["check:migration-billing-multi-psp-e2e"]) {
		errors.push("check:migration-billing-multi-psp-e2e: package script is required");
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Billing Multi-PSP E2E Evidence",
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
		"- PSP webhook intake must live in billing-service, not Account.",
		"- PSP webhook processing, analytics, emails and dunning must live in Billing worker.",
		"- The repository smoke proof must be deterministic and must not call external PSP APIs.",
		"- Multi-PSP continuity must cover Stripe, Mollie, CB, primary ownership and fallback eligibility.",
		"",
		"## Evidence",
		"",
		"| Check | Status | Path |",
		"|---|---:|---|",
	];
	for (const check of data.checks) lines.push(`| ${check.description} | ${check.status} | \`${check.path}\` |`);
	lines.push(
		"",
		"## Decision",
		"",
		data.summary.failed === 0
			? "Billing multi-PSP repository E2E evidence is covered without external PSP dependencies. Production cutover still requires live environment evidence."
			: "Billing multi-PSP repository E2E evidence is blocked until failed checks pass.",
		"",
		"## Verification",
		"",
		"```bash",
		"pnpm check:migration-billing-multi-psp-e2e",
		"```",
		"",
		"## Regeneration",
		"",
		"```bash",
		"node tools/migration/billing-multi-psp-e2e.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

const checks = buildChecks();
const report = {
	schema_version: 1,
	generation: {
		command: "node tools/migration/billing-multi-psp-e2e.mjs --write",
		sources: Object.values(sources),
		targeted_tests: targetedTests,
	},
	summary: summarize(checks),
	checks,
};

validateReport(report);

const json = serializeJson(report);
const markdown = serializeMarkdown(report);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Billing multi-PSP E2E evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}
for (const [path, expected] of [[outputPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/billing-multi-psp-e2e.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/billing-multi-psp-e2e.mjs --write`);
}

if (errors.length > 0) {
	console.error("Billing multi-PSP E2E evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Billing multi-PSP E2E evidence: ok (${report.summary.passed}/${report.summary.checks} checks passed)`);
