#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/identity-login-session.generated.json";
const markdownPath = "docs/migration/identity-login-session.md";
const sources = {
	loginSession: "apps/account-service/src/identity.domains.auth.sessions.create.session.rs",
	loginVerify: "apps/account-service/src/identity.domains.auth.sessions.create.verify.rs",
	loginPwdRoute: "apps/account-service/src/identity.domains.auth.routes.login.pwd.rs",
	loginIdentifierRoute: "apps/account-service/src/identity.domains.auth.routes.login.identifier.rs",
	sessionMgmt: "apps/account-service/src/identity.domains.auth.sessions.mgmt.rs",
	sessionMgmtTests: "apps/account-service/src/identity.domains.auth.sessions.mgmt.tests.rs",
	sessionRefresh: "apps/account-service/src/identity.http.middleware.jwt.session_refresh.rs",
	openapi: "apps/account-service/openapi.json",
	contractSmoke: "scripts/test-openapi-contract.mjs",
	riskRegister: "tools/migration/risk-register.evidence.mjs",
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
		textCheck("identifier-route", sources.loginIdentifierRoute, "Login identifier route is present", "challenge_identifier"),
		textCheck("password-route", sources.loginPwdRoute, "Password login route is present", "challenge_pwd"),
		textCheck("login-throttle", sources.loginVerify, "Primary login applies throttle rules", "LOGIN_THROTTLE_ACTION"),
		textCheck("password-check", sources.loginVerify, "Primary login verifies stored password", "verify_password(&stored_hash, &input.password)"),
		textCheck("login-failed-risk", sources.loginVerify, "Failed login records risk event", "\"login_failed\""),
		textCheck("login-failed-audit", sources.loginVerify, "Failed login records audit event", "\"auth.login_failed\""),
		textCheck("session-token", sources.loginSession, "Successful login issues a session-bound token", "generate_token_pair_with_session"),
		textCheck("session-cache", sources.loginSession, "Successful login stores cached session", "set_session(redis, &cached_session"),
		textCheck("login-success-risk", sources.loginSession, "Successful login records risk event", "\"login_success\""),
		textCheck("login-success-audit", sources.loginSession, "Successful login records audit event", "\"auth.login_success\""),
		textCheck("logout-route", sources.openapi, "OpenAPI contains logout route", "\"/auth/logout\""),
		textCheck("sessions-route", sources.openapi, "OpenAPI contains session listing route", "\"/auth/sessions\""),
		textCheck("logout-state-helper", sources.sessionMgmt, "Logout revocation state is isolated from audit persistence", "revoke_logout_session_state"),
		textCheck("logout-revokes-refresh", sources.sessionMgmt, "Logout revokes refresh tokens for the current session", "revoke_session_refresh_tokens"),
		textCheck("logout-deletes-session", sources.sessionMgmt, "Logout deletes the current session cache", "delete_session(redis, &user_id.to_string(), &session_id.to_string())"),
		textCheck("logout-audit", sources.sessionMgmt, "Logout records session revoked audit event", "\"auth.session_revoked\""),
		textCheck("logout-test", sources.sessionMgmtTests, "Logout state test proves only current session and refresh token are revoked", "logout_state_revokes_current_session_and_refresh_token_only"),
		textCheck("refresh-entrypoint", sources.sessionRefresh, "Refresh middleware retries expired access tokens", "refresh_expired_session"),
		textCheck("refresh-session-guard", sources.sessionRefresh, "Refresh rejects revoked or expired cached sessions", "session.revoked_at.is_some()"),
		textCheck("refresh-token-rotation", sources.sessionRefresh, "Refresh rotates cached access-token hash", "update_session_token_hash"),
		textCheck("refresh-cookie-helper", sources.sessionRefresh, "Refresh cookie construction is pure-testable", "build_refreshed_cookies_from_config"),
		textCheck("refresh-cookie-test", sources.sessionRefresh, "Refresh cookie tests cover authuser-scoped secure cookies", "refreshed_cookies_are_authuser_scoped_and_secure_outside_development"),
		textCheck("seeded-login-identifier", sources.contractSmoke, "Seeded OpenAPI smoke calls login identifier challenge", "/auth/challenge/identifier"),
		textCheck("seeded-login-password", sources.contractSmoke, "Seeded OpenAPI smoke calls password challenge", "/auth/challenge/pwd"),
		textCheck("seeded-session-cookie", sources.contractSmoke, "Seeded OpenAPI smoke requires a session cookie", "did not set a session cookie"),
		textCheck("seeded-session-list", sources.contractSmoke, "Seeded OpenAPI smoke lists sessions after login", "/auth/sessions"),
		textCheck("session-migration-decision", sources.riskRegister, "Session migration risk has explicit clearing/login return decision", "explicit logout/session clearing and a validated login return flow"),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in identity login/session source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "tools/migration/identity-login-session.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match identity login/session source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"cargo test -p nvbes-account-service refreshed_cookies --locked",
		"cargo test -p nvbes-account-service logout_state_revokes_current_session_and_refresh_token_only --locked",
		"bash scripts/test-openapi-contract-seeded-auth.sh",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "identity-login-session", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Identity Login Session Evidence",
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
		"- Every evidence row must be generated from the identity login/session source contract.",
		"- `passed` requires the configured source, OpenAPI document, or migration risk source to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name refresh, logout, and seeded auth smoke checks required by parity.",
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
			? "Identity login/logout/refresh parity evidence is covered for repository cutover gates."
			: "Identity login/logout/refresh parity evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-identity-login-session",
		"tools/migration/identity-login-session.mjs --write",
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
		command: "tools/migration/identity-login-session.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"cargo test -p nvbes-account-service refreshed_cookies --locked",
			"cargo test -p nvbes-account-service logout_state_revokes_current_session_and_refresh_token_only --locked",
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
	console.log(`Identity login session evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/identity-login-session.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/identity-login-session.mjs --write`);
}

if (errors.length > 0) {
	console.error("Identity login session evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Identity login session evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
