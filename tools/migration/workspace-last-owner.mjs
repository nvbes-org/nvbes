#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/workspace-last-owner.generated.json";
const markdownPath = "docs/migration/workspace-last-owner.md";
const sources = {
	dbOwners: "apps/account-service/src/identity.domains.enterprise.db.owners.rs",
	userMutations: "apps/account-service/src/identity.domains.enterprise.service.user_mutations.rs",
	accessRevocations: "apps/account-service/src/identity.domains.enterprise.access_reviews.revocations.rs",
	accessRevocationTests: "apps/account-service/src/identity.domains.enterprise.access_reviews.revocations.tests.rs",
	corePolicyTests: "libs/rust/core/src/authz.policy.tests.rs",
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
		textCheck("owner-change-lock", sources.dbOwners, "Owner changes use tenant-scoped advisory transaction lock", "pg_advisory_xact_lock"),
		textCheck("ownerless-access-query", sources.dbOwners, "Role updates count workspaces that would lose their last owner", "ownerless_workspace_count_after_access"),
		textCheck("ownerless-status-query", sources.dbOwners, "Lifecycle updates count workspaces that would lose their last owner", "ownerless_workspace_count_after_status"),
		textCheck("ownerless-other-owner-check", sources.dbOwners, "Ownerless queries require absence of another active owner", "NOT EXISTS (\n                    SELECT 1\n                    FROM workspace_memberships other_owner"),
		textCheck("update-access-lock", sources.userMutations, "Enterprise access updates lock owner changes before mutation", "lock_tenant_owner_changes(&mut tx, tenant_id)"),
		textCheck("update-access-ownerless-check", sources.userMutations, "Enterprise access updates check ownerless workspaces before replace", "ownerless_workspace_count_after_access"),
		textCheck("update-access-conflict", sources.userMutations, "Enterprise access updates reject last-owner removal", "\"last_owner_removal\""),
		textCheck("update-access-audit", sources.userMutations, "Enterprise access updates audit membership changes", "\"enterprise.member.access_updated\""),
		textCheck("suspend-ownerless-check", sources.userMutations, "Enterprise suspension checks ownerless workspaces before status change", "ownerless_workspace_count_after_status"),
		textCheck("suspend-audit", sources.userMutations, "Enterprise suspension audits membership changes", "\"enterprise.member.suspended\""),
		textCheck("review-member-lock", sources.accessRevocations, "Access review member revocation locks owner changes", "lock_tenant_owner_changes(tx, tenant_id).await?"),
		textCheck("review-member-guard", sources.accessRevocations, "Access review member revocation checks tenant last owner", "ensure_not_last_owner(tx, tenant_id, principal_id).await?"),
		textCheck("review-role-guard", sources.accessRevocations, "Access review role revocation checks workspace last owner", "ensure_not_last_workspace_owner(tx, tenant_id, principal_id, workspace_id).await?"),
		textCheck("review-service-account-guard", sources.accessRevocations, "Access review service-account revocation checks workspace last owner", "ensure_not_last_workspace_owner(tx, tenant_id, principal_id, workspace_id).await?"),
		textCheck("guard-conflict-code", sources.accessRevocations, "Last-owner service guard returns stable conflict code", "\"last_owner_removal\""),
		textCheck("guard-pass-test", sources.accessRevocationTests, "Service test allows non-ownerless mutation", "last_owner_guard_allows_when_no_workspace_would_be_ownerless"),
		textCheck("guard-block-test", sources.accessRevocationTests, "Service test blocks ownerless mutation", "last_owner_guard_blocks_ownerless_workspace"),
		textCheck("owner-target-policy-test", sources.corePolicyTests, "Core authz test prevents owner-to-owner invite/remove actions", "owner_cannot_invite_or_remove_another_owner"),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in workspace last-owner source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "tools/migration/workspace-last-owner.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match workspace last-owner source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"cargo test -p nvbes-account-service last_owner_guard --locked",
		"cargo test -p nvbes-core owner_cannot_invite_or_remove_another_owner --locked",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "workspace-last-owner", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Workspace Last-Owner Evidence",
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
		"- Every evidence row must be generated from the workspace last-owner source contract.",
		"- `passed` requires the configured source file to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name last-owner guard and owner policy checks required by parity.",
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
			? "Workspace/Authz last-owner protection evidence is covered for repository cutover gates."
			: "Workspace/Authz last-owner protection evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-workspace-last-owner",
		"tools/migration/workspace-last-owner.mjs --write",
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
		command: "tools/migration/workspace-last-owner.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"cargo test -p nvbes-account-service last_owner_guard --locked",
			"cargo test -p nvbes-core owner_cannot_invite_or_remove_another_owner --locked",
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
	console.log(`Workspace last-owner evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/workspace-last-owner.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/workspace-last-owner.mjs --write`);
}

if (errors.length > 0) {
	console.error("Workspace last-owner evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Workspace last-owner evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
