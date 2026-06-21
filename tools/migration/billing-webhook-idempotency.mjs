#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/billing-webhook-idempotency.generated.json";
const markdownPath = "docs/migration/billing-webhook-idempotency.md";

const sources = {
	openapi: "apps/identity-api/openapi.json",
	route: "apps/identity-api/src/identity.domains.billing.routes.webhooks.rs",
	handler: "apps/identity-api/src/identity.domains.billing.webhooks.handlers.rs",
	logic: "apps/identity-api/src/identity.domains.billing.webhooks.logic.rs",
	logicTests: "apps/identity-api/src/identity.domains.billing.webhooks.tests.logic.rs",
	jobs: "apps/identity-api/src/identity.domains.billing.jobs.rs",
	migration: "apps/identity-api/migrations/0001_initial_schema.sql",
	openapiExport: "apps/identity-api/src/identity.http.openapi.rs",
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

function parseJson(path, content) {
	try {
		return JSON.parse(content);
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return undefined;
	}
}

function openApiPathCheck() {
	const content = read(sources.openapi);
	if (!content) {
		return {
			id: "openapi-stripe-webhook-path",
			description: "OpenAPI exposes POST /webhooks/stripe",
			path: sources.openapi,
			status: "failed",
			pattern: "/webhooks/stripe",
		};
	}
	const api = parseJson(sources.openapi, content);
	return {
		id: "openapi-stripe-webhook-path",
		description: "OpenAPI exposes POST /webhooks/stripe",
		path: sources.openapi,
		status: api.paths?.["/webhooks/stripe"]?.post ? "passed" : "failed",
		pattern: "paths['/webhooks/stripe'].post",
	};
}

function buildChecks() {
	return [
		openApiPathCheck(),
		textCheck("openapi-export-route", sources.openapiExport, "OpenAPI export includes Stripe webhook handler", "crate::domains::billing::routes::webhooks::handle_stripe_webhook"),
		textCheck("route-handler", sources.route, "Axum route accepts Stripe webhooks", 'route("/webhooks/stripe", post(handle_stripe_webhook))'),
		textCheck("rate-limit", sources.route, "Webhook route enforces rate limiting before processing", "enforce_stripe_webhook_rate_limit"),
		textCheck("provider-event-lock", sources.handler, "Handler locks existing provider event rows", "WHERE provider_event_id = $1\n        FOR UPDATE"),
		textCheck("duplicate-decision", sources.logic, "Processed webhooks are treated as duplicates", 'Some("processed") => WebhookRetryDecision::Duplicate'),
		textCheck("replay-failed-decision", sources.logic, "Failed or received webhooks are replayable", 'Some("failed" | "received") => WebhookRetryDecision::ReplayFailed'),
		textCheck("conflict-idempotency", sources.handler, "Insert path is idempotent on provider_event_id", "ON CONFLICT (provider_event_id) DO NOTHING"),
		textCheck("duplicate-response", sources.handler, "Duplicate path returns duplicate without enqueueing", 'status: "duplicate".to_string()'),
		textCheck("replay-update", sources.handler, "Replay path resets failed or received events", "status = 'received'"),
		textCheck("queue-idempotency-key", sources.jobs, "Queued webhook job uses provider_event_id as idempotency key", "idempotency_key: Some(provider_event_id.to_string())"),
		textCheck("event-id-unique", sources.migration, "Database constrains provider_event_id uniqueness", "provider_event_id TEXT NOT NULL UNIQUE"),
		textCheck("failed-replay-test", sources.logicTests, "Unit test covers failed replay classification", "classify_webhook_retry_treats_failed_as_replayable"),
		textCheck("received-replay-test", sources.logicTests, "Unit test covers received replay classification", "classify_webhook_retry_treats_received_as_replayable"),
		textCheck("duplicate-test", sources.logicTests, "Unit test covers processed duplicate classification", "classify_webhook_retry_treats_processed_as_duplicate"),
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
	if (report.generation?.command !== "tools/migration/billing-webhook-idempotency.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match billing webhook source contract`);
	}
	if (report.generation?.targeted_test !== "cargo test -p nvbes-identity-api classify_webhook_retry --locked") {
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
		"- `passed` requires the configured file or OpenAPI document to contain the expected pattern.",
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
		"tools/migration/billing-webhook-idempotency.mjs --write",
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
		command: "tools/migration/billing-webhook-idempotency.mjs --write",
		sources: Object.values(sources),
		targeted_test: "cargo test -p nvbes-identity-api classify_webhook_retry --locked",
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
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/billing-webhook-idempotency.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/billing-webhook-idempotency.mjs --write`);
}

if (errors.length > 0) {
	console.error("Billing webhook idempotency evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Billing webhook idempotency evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
