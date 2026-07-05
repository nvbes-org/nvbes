#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/identity-register.generated.json";
const markdownPath = "docs/migration/identity-register.md";
const sources = {
	registerRoute: "apps/account-service/src/identity.domains.auth.routes.register.rs",
	registerRouteTests: "apps/account-service/src/identity.domains.auth.routes.register.tests.rs",
	onboarding: "apps/account-service/src/identity.domains.auth.onboarding.rs",
	accountDb: "apps/account-service/src/identity.domains.auth.db.account.rs",
	authRoutes: "apps/account-service/src/identity.domains.auth.routes.rs",
	openapiSource: "apps/account-service/src/identity.http.openapi.rs",
	openapiJson: "apps/account-service/openapi.json",
	contractSmoke: "scripts/test-openapi-contract.mjs",
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
		textCheck("route-mounted", sources.authRoutes, "Auth routes mount register under /auth", ".nest(\"/auth\", register::router())"),
		textCheck("register-route", sources.registerRoute, "Register route accepts POST /register", ".route(\"/register\", post(register))"),
		textCheck("openapi-annotation", sources.registerRoute, "Register route declares /auth/register OpenAPI path", "path = \"/auth/register\""),
		textCheck("request-body", sources.registerRoute, "Register route declares RegisterRequest body", "request_body = RegisterRequest"),
		textCheck("success-response", sources.registerRoute, "Register route returns RegisterResult", "body = crate::domains::auth::types::RegisterResult"),
		textCheck("pow-gate", sources.registerRoute, "Register route enforces proof-of-work when enabled", "require_pow_solution"),
		textCheck("rate-limit", sources.registerRoute, "Register route applies dual rate limit", "\"auth_register\""),
		textCheck("region-resolution", sources.registerRoute, "Register route resolves supported country to data region", "country_code_to_data_region"),
		textCheck("onboarding-mapping", sources.registerRoute, "Register route maps HTTP request to onboarding input through a tested helper", "register_input_from_request"),
		textCheck("mapping-test", sources.registerRouteTests, "Register route mapping test covers data region, workspace, IP, and user-agent", "register_input_from_request_maps_http_payload_to_onboarding_input"),
		textCheck("onboarding-register", sources.onboarding, "Onboarding exposes register command", "pub async fn register"),
		textCheck("email-validation", sources.onboarding, "Onboarding validates normalized email", "validate_email(&email)?"),
		textCheck("password-validation", sources.onboarding, "Onboarding validates password policy", "validate_password(&input.password)?"),
		textCheck("register-limiter", sources.onboarding, "Onboarding applies account-level register rate limit", "\"register\""),
		textCheck("create-account", sources.onboarding, "Onboarding creates the user account transaction", "db::create_user_account"),
		textCheck("password-history", sources.onboarding, "Onboarding records password history", "history::insert_password_hash"),
		textCheck("tenant-insert", sources.accountDb, "Registration creates personal tenant", "INSERT INTO tenants"),
		textCheck("principal-insert", sources.accountDb, "Registration creates human principal", "INSERT INTO principals"),
		textCheck("user-insert", sources.accountDb, "Registration creates pending verification user", "'pending_verification'"),
		textCheck("workspace-insert", sources.accountDb, "Registration creates personal workspace", "INSERT INTO workspaces"),
		textCheck("workspace-policy", sources.accountDb, "Registration creates workspace policy", "INSERT INTO workspace_policies"),
		textCheck("owner-membership", sources.accountDb, "Registration creates owner workspace membership", "INSERT INTO workspace_memberships"),
		textCheck("audit-event", sources.accountDb, "Registration writes user.registered audit event", 'action: "user.registered"'),
		textCheck("verification-email", sources.accountDb, "Registration issues verification email token", "issue_verification_email_tx"),
		textCheck("openapi-source", sources.openapiSource, "OpenAPI source exports register path", "crate::domains::auth::routes::register::register"),
		textCheck("openapi-json-path", sources.openapiJson, "Generated OpenAPI contains /auth/register", "\"/auth/register\""),
		textCheck("openapi-json-operation", sources.openapiJson, "Generated OpenAPI exposes register operation id", "\"operationId\":\"register\""),
		textCheck("seeded-smoke-register", sources.contractSmoke, "Seeded OpenAPI smoke registers an account before login", "`${apiBaseUrl}/auth/register`"),
		textCheck("seeded-smoke-status", sources.contractSmoke, "Seeded OpenAPI smoke expects register success or idempotent conflict", "[200, 409]"),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in identity register source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "tools/migration/identity-register.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match identity register source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"cargo test -p nvbes-account-service register_input_from_request --locked",
		"bash scripts/test-openapi-contract-seeded-auth.sh",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "identity-register", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Identity Register Evidence",
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
		"- Every evidence row must be generated from the identity register source contract.",
		"- `passed` requires the configured file to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name the register checks required by parity.",
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
			? "Identity register parity evidence is covered for repository cutover gates."
			: "Identity register parity evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-identity-register",
		"tools/migration/identity-register.mjs --write",
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
		command: "tools/migration/identity-register.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"cargo test -p nvbes-account-service register_input_from_request --locked",
			"bash scripts/test-openapi-contract-seeded-auth.sh",
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
	console.log(`Identity register evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/identity-register.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/identity-register.mjs --write`);
}

if (errors.length > 0) {
	console.error("Identity register evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Identity register evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
