#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/identity-mfa-webauthn.generated.json";
const markdownPath = "docs/migration/identity-mfa-webauthn.md";
const sources = {
	mfaRoot: "apps/account-service/src/identity.domains.auth.routes.mfa.rs",
	mfaModule: "apps/account-service/src/identity.domains.auth.mfa.rs",
	mfaFlow: "apps/account-service/src/identity.domains.auth.routes.login.mfa_flow.rs",
	loginMfaRoute: "apps/account-service/src/identity.domains.auth.routes.login.mfa.rs",
	loginWebauthnRoute: "apps/account-service/src/identity.domains.auth.routes.login.webauthn.rs",
	totpRoute: "apps/account-service/src/identity.domains.auth.routes.mfa.totp.rs",
	totpService: "apps/account-service/src/identity.domains.auth.mfa.totp.rs",
	recoveryService: "apps/account-service/src/identity.domains.auth.mfa.recovery.rs",
	webauthnRoute: "apps/account-service/src/identity.domains.auth.routes.mfa.webauthn.rs",
	webauthnRegistrationStart: "apps/account-service/src/identity.domains.auth.webauthn.registration.start.rs",
	webauthnRegistrationFinish: "apps/account-service/src/identity.domains.auth.webauthn.registration.finish.rs",
	webauthnRegistrationOptions: "apps/account-service/src/identity.domains.auth.webauthn.registration.options.rs",
	webauthnRegistrationTests: "apps/account-service/src/identity.domains.auth.webauthn.registration.tests.rs",
	webauthnLogin: "apps/account-service/src/identity.domains.auth.webauthn.login.rs",
	webauthnDiscoverableLogin: "apps/account-service/src/identity.domains.auth.webauthn.login.discoverable.rs",
	webauthnStepUp: "apps/account-service/src/identity.domains.auth.webauthn.authentication.rs",
	openapiSource: "apps/account-service/src/identity.http.openapi.rs",
	openapiJson: "apps/account-service/openapi.json",
	contractSmoke: "scripts/test-openapi-contract.mjs",
	dataMap: "docs/migration/data-map.md",
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
		textCheck("mfa-router", sources.mfaRoot, "MFA router exposes factor list and enrollment routes", ".merge(webauthn::router(state))"),
		textCheck("totp-setup-route", sources.totpRoute, "TOTP setup route is protected by JWT and step-up", "require_recent_step_up"),
		textCheck("totp-confirm-route", sources.totpRoute, "TOTP confirm route activates a factor", "confirm_totp"),
		textCheck("totp-secret", sources.totpService, "TOTP enrollment generates a secret through core MFA primitives", "generate_totp_secret()"),
		textCheck("totp-replay-guard", sources.totpService, "TOTP verification rejects replayed counters", "counter as i64 > last_counter"),
		textCheck("recovery-generate", sources.recoveryService, "Recovery code generation stores hashed codes", "codes.iter().map(|code| token_hash(code))"),
		textCheck("recovery-one-time", sources.recoveryService, "Recovery verification consumes one matching code", "new_codes.push(c.clone())"),
		textCheck("login-methods-test", sources.mfaModule, "Login method ordering is covered by a unit test", "login_methods_are_ordered_by_preference"),
		textCheck("mfa-login-route", sources.loginMfaRoute, "MFA login challenge route is exposed", "path = \"/auth/challenge/mfa\""),
		textCheck("mfa-rate-limit", sources.loginMfaRoute, "MFA login challenge is rate limited by IP and state", "\"auth_login_mfa\""),
		textCheck("mfa-single-factor", sources.mfaFlow, "MFA login accepts exactly one factor", "validate_single_factor"),
		textCheck("mfa-single-factor-test", sources.mfaFlow, "MFA login rejects multiple factors in a deterministic unit test", "validate_single_factor_rejects_multiple_factors"),
		textCheck("mfa-failed-risk", sources.loginMfaRoute, "MFA failures record risk events", "\"mfa_failed\""),
		textCheck("mfa-failed-audit", sources.loginMfaRoute, "MFA failures record audit events", "\"auth.mfa_failed\""),
		textCheck("mfa-aal2-session", sources.loginMfaRoute, "Successful MFA creates an AAL2 session", "\"aal2\""),
		textCheck("login-webauthn-start", sources.loginWebauthnRoute, "Passwordless WebAuthn start route is exposed", "path = \"/auth/challenge/webauthn/start\""),
		textCheck("login-webauthn-discoverable-start", sources.loginWebauthnRoute, "Discoverable WebAuthn start route is exposed", "path = \"/auth/challenge/webauthn/discoverable/start\""),
		textCheck("login-webauthn-discoverable-finish", sources.loginWebauthnRoute, "Discoverable WebAuthn finish route creates an AAL2 session", "challenge_webauthn_discoverable_finish"),
		textCheck("enroll-webauthn-start", sources.webauthnRoute, "WebAuthn enrollment start route is exposed", "path = \"/auth/mfa/webauthn/register/start\""),
		textCheck("enroll-webauthn-step-up", sources.webauthnRoute, "WebAuthn enrollment requires recent AAL2 step-up", "require_recent_step_up"),
		textCheck("enroll-webauthn-finish", sources.webauthnRoute, "WebAuthn enrollment finish route is exposed", "path = \"/auth/mfa/webauthn/register/finish\""),
		textCheck("webauthn-registration-state", sources.webauthnRegistrationStart, "WebAuthn registration challenge state is cached", "purpose: \"webauthn_registration\""),
		textCheck("webauthn-registration-audit-start", sources.webauthnRegistrationStart, "WebAuthn registration start is audited", "\"webauthn_registration_started\""),
		textCheck("webauthn-registration-finish", sources.webauthnRegistrationFinish, "WebAuthn registration finish persists passkey data", "\"passkey\""),
		textCheck("webauthn-registration-audit-finish", sources.webauthnRegistrationFinish, "WebAuthn registration finish is audited", "\"webauthn_registered\""),
		textCheck("webauthn-options", sources.webauthnRegistrationOptions, "WebAuthn registration distinguishes passkeys from security keys", "\"security_key\""),
		textCheck("webauthn-options-test", sources.webauthnRegistrationTests, "WebAuthn option shaping is covered by unit tests", "shape_registration_options_sets_cross_platform_attachment_for_security_keys"),
		textCheck("webauthn-login-challenge", sources.webauthnLogin, "WebAuthn login stores challenge state", "purpose: \"webauthn_login\""),
		textCheck("webauthn-login-success", sources.webauthnLogin, "WebAuthn login success is audited", "\"webauthn_login_authenticated\""),
		textCheck("webauthn-discoverable-success", sources.webauthnDiscoverableLogin, "Discoverable WebAuthn login success is audited", "\"webauthn_discoverable_login_authenticated\""),
		textCheck("webauthn-step-up", sources.webauthnStepUp, "WebAuthn step-up challenge is supported", "purpose: \"webauthn_step_up\""),
		textCheck("openapi-source", sources.openapiSource, "OpenAPI source includes MFA and WebAuthn routes", "crate::domains::auth::routes::mfa::begin_webauthn_enrollment"),
		textCheck("openapi-mfa-path", sources.openapiJson, "Generated OpenAPI includes MFA challenge", "\"/auth/challenge/mfa\""),
		textCheck("openapi-webauthn-path", sources.openapiJson, "Generated OpenAPI includes WebAuthn enrollment", "\"/auth/mfa/webauthn/register/start\""),
		textCheck("seeded-smoke-mfa-profile", sources.contractSmoke, "Seeded OpenAPI smoke observes mfa_enabled in login profile", "mfaEnabled: Boolean(loginBody?.user?.mfa_enabled)"),
		textCheck("identity-mfa-rebuild-decision", sources.dataMap, "Identity MFA credentials have an explicit rebuild migration decision", "`target-postgres:identity.mfa_factors`"),
		textCheck("legacy-drive-mfa-rebuild-decision", sources.dataMap, "Legacy Drive MFA credentials have an explicit rebuild migration decision", "`target-postgres:drive.mfa_factors`"),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in identity MFA/WebAuthn source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "node tools/migration/identity-mfa-webauthn.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match identity MFA/WebAuthn source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"cargo test -p nvbes-account-service validate_single_factor --locked",
		"cargo test -p nvbes-account-service shape_registration_options --locked",
		"cargo test -p nvbes-account-service login_methods_are_ordered_by_preference --locked",
		"bash scripts/test-openapi-contract-seeded-auth.sh",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "identity-mfa-webauthn", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Identity MFA WebAuthn Evidence",
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
		"- Every evidence row must be generated from the identity MFA/WebAuthn source contract.",
		"- `passed` requires the configured source, OpenAPI document, smoke test, or migration map to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name MFA, WebAuthn option shaping, login-method ordering, and seeded auth smoke checks required by parity.",
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
			? "Identity MFA/WebAuthn parity evidence is covered for repository cutover gates. Credential migration remains a rebuild decision documented in the data map."
			: "Identity MFA/WebAuthn parity evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-identity-mfa-webauthn",
		"node tools/migration/identity-mfa-webauthn.mjs --write",
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
		command: "node tools/migration/identity-mfa-webauthn.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"cargo test -p nvbes-account-service validate_single_factor --locked",
			"cargo test -p nvbes-account-service shape_registration_options --locked",
			"cargo test -p nvbes-account-service login_methods_are_ordered_by_preference --locked",
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
	console.log(`Identity MFA/WebAuthn evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/identity-mfa-webauthn.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/identity-mfa-webauthn.mjs --write`);
}

if (errors.length > 0) {
	console.error("Identity MFA/WebAuthn evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Identity MFA/WebAuthn evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
