#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/workspace-membership-roles.generated.json";
const markdownPath = "docs/migration/workspace-membership-roles.md";
const sources = {
	corePolicy: "libs/rust/core/src/authz.policy.rs",
	corePolicyTests: "libs/rust/core/src/authz.policy.tests.rs",
	coreRole: "libs/rust/core/src/authz.role.rs",
	authzDb: "apps/account-service/src/identity.domains.authz.db.rs",
	authzService: "apps/account-service/src/identity.domains.authz.service.rs",
	authzRoutes: "apps/account-service/src/identity.domains.authz.routes.rs",
	openapi: "apps/account-service/openapi.json",
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
		textCheck("owner-role", sources.coreRole, "Owner role is part of the workspace role model", "WorkspaceRole::Owner"),
		textCheck("admin-role", sources.coreRole, "Admin role is part of the workspace role model", "WorkspaceRole::Admin"),
		textCheck("security-admin-role", sources.coreRole, "Security admin role is part of the workspace role model", "WorkspaceRole::SecurityAdmin"),
		textCheck("billing-admin-role", sources.coreRole, "Billing admin role is part of the workspace role model", "WorkspaceRole::BillingAdmin"),
		textCheck("member-role", sources.coreRole, "Member role is part of the workspace role model", "WorkspaceRole::Member"),
		textCheck("viewer-role", sources.coreRole, "Viewer role is part of the workspace role model", "WorkspaceRole::Viewer"),
		textCheck("policy-entrypoint", sources.corePolicy, "Workspace role policy uses the shared is_allowed entrypoint", "pub fn is_allowed"),
		textCheck("owner-policy", sources.corePolicy, "Owner policy is explicit", "fn owner_allows"),
		textCheck("admin-policy", sources.corePolicy, "Admin policy is explicit", "fn admin_allows"),
		textCheck("security-admin-policy", sources.corePolicy, "Security admin policy is explicit", "fn security_admin_allows"),
		textCheck("billing-admin-policy", sources.corePolicy, "Billing admin policy is explicit", "fn billing_admin_allows"),
		textCheck("member-policy", sources.corePolicy, "Member policy is explicit", "fn member_allows"),
		textCheck("viewer-policy", sources.corePolicy, "Viewer policy is explicit", "fn viewer_allows"),
		textCheck("role-boundary-test", sources.corePolicyTests, "Core test covers owner/admin/security/billing/member/viewer permission boundaries", "workspace_membership_roles_cover_expected_permission_boundaries"),
		textCheck("admin-invite-test", sources.corePolicyTests, "Core test blocks admin escalation through invites", "admin_can_only_invite_member_or_viewer"),
		textCheck("security-admin-test", sources.corePolicyTests, "Core test limits security admin to audit/security views", "security_admin_can_view_audit_but_cannot_invite_members"),
		textCheck("billing-admin-test", sources.corePolicyTests, "Core test limits billing admin away from audit", "billing_admin_can_manage_billing_but_cannot_view_audit"),
		textCheck("viewer-readonly-test", sources.corePolicyTests, "Core test keeps viewer read-only", "viewer_is_read_only"),
		textCheck("service-policy-call", sources.authzService, "Account Service authorization calls the shared role policy", "is_allowed(access.role, action, effective_resource)"),
		textCheck("denied-audit-call", sources.authzService, "Denied workspace actions are audited before returning an error", "record_permission_denied(db, &access, action, effective_resource, headers).await?"),
		textCheck("audit-insert", sources.authzDb, "Permission denied decisions insert an audit event", "record_event_tx"),
		textCheck("audit-action", sources.authzDb, "Permission denied audit uses stable action literal", 'action: "permission.denied"'),
		textCheck("audit-metadata-helper", sources.authzDb, "Permission denied audit metadata is generated through a tested helper", "permission_denied_metadata"),
		textCheck("audit-metadata-test", sources.authzDb, "Permission denied audit metadata has a unit test", "permission_denied_metadata_captures_role_action_and_target_role"),
		textCheck("decision-route", sources.authzRoutes, "Authz decision endpoint is routed", "/authz/decision"),
		textCheck("decision-openapi", sources.openapi, "Authz decision endpoint is present in OpenAPI", "\"/authz/decision\""),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in workspace membership source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "node tools/migration/workspace-membership-roles.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match workspace membership source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"cargo test -p nvbes-core workspace_membership_roles_cover_expected_permission_boundaries --locked",
		"cargo test -p nvbes-account-service permission_denied_metadata_captures_role_action_and_target_role --locked",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "workspace-membership-roles", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Workspace Membership Roles Evidence",
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
		"- Every evidence row must be generated from the workspace membership source contract.",
		"- `passed` requires the configured source or OpenAPI document to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name role-boundary and denied-audit metadata checks required by parity.",
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
			? "Workspace/Authz membership role parity evidence is covered for repository cutover gates."
			: "Workspace/Authz membership role parity evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-workspace-membership-roles",
		"node tools/migration/workspace-membership-roles.mjs --write",
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
		command: "node tools/migration/workspace-membership-roles.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"cargo test -p nvbes-core workspace_membership_roles_cover_expected_permission_boundaries --locked",
			"cargo test -p nvbes-account-service permission_denied_metadata_captures_role_action_and_target_role --locked",
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
	console.log(`Workspace membership roles evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/workspace-membership-roles.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/workspace-membership-roles.mjs --write`);
}

if (errors.length > 0) {
	console.error("Workspace membership roles evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Workspace membership roles evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
