#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/frontend-experience.generated.json";
const markdownPath = "docs/migration/frontend-experience.md";

const sources = {
	packageJson: "package.json",
	identityRouter: "apps/identity-web/src/identity.router.tsx",
	identityAccountLayout: "apps/identity-web/src/components/AccountLayout.tsx",
	identityCriticalE2e: "apps/identity-web/e2e/critical.spec.ts",
	identityUniversalLoginTest: "apps/identity-web/src/identity.universal-login.test.js",
	identityUniversalLoginHookTest: "apps/identity-web/src/pages/useUniversalLogin.test.js",
	driveRouter: "apps/drive-web/src/drive.router.tsx",
	driveLayout: "apps/drive-web/src/DriveAppLayout.tsx",
	driveSharedLinks: "apps/drive-web/src/DriveSharedLinksView.tsx",
	driveUploadTest: "apps/drive-web/src/drive.uploads.drop.test.ts",
	driveSessionTest: "apps/drive-web/src/drive.session.storage.test.ts",
	driveWorkspaceTest: "apps/drive-web/src/drive.workspace.store.test.ts",
	developerRouter: "apps/developer-web/src/developer.router.tsx",
	developerShell: "apps/developer-web/src/layouts/DeveloperShell.tsx",
	developerPortalLayout: "apps/developer-web/src/layouts/DeveloperPortalLayout.tsx",
	developerPublicLayout: "apps/developer-web/src/layouts/DeveloperPublicLayout.tsx",
	developerPermissionsTest: "apps/developer-web/src/__tests__/developer.permissions.test.ts",
	developerSessionTest: "apps/developer-web/src/__tests__/developer.session.test.ts",
	developerWebhooksTest: "apps/developer-web/src/__tests__/developer.webhooks.test.ts",
	enterpriseRouter: "apps/enterprise-web/src/enterprise.router.tsx",
	enterpriseLayout: "apps/enterprise-web/src/components/EnterpriseLayout.tsx",
	enterpriseSidebar: "apps/enterprise-web/src/components/EnterpriseSidebar.tsx",
	enterprisePermissionsTest: "apps/enterprise-web/src/enterprise.permissions.test.ts",
	enterpriseInvitesTest: "apps/enterprise-web/src/enterprise.invites.test.ts",
	cloudConsoleReadme: "apps/cloud-console/README.md",
	internalAdminReadme: "apps/internal-admin/README.md",
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

function packageScriptCheck(id, scriptName, description, pattern) {
	const content = read(sources.packageJson);
	if (!content) {
		return { id, description, path: sources.packageJson, status: "failed", pattern };
	}
	const scripts = JSON.parse(content).scripts ?? {};
	const command = scripts[scriptName] ?? "";
	return {
		id,
		description,
		path: sources.packageJson,
		status: command.includes(pattern) ? "passed" : "failed",
		pattern: `${scriptName}: ${pattern}`,
	};
}

function buildChecks() {
	return [
		packageScriptCheck("root-check-web-enterprise-format", "check:web", "Root web check formats Enterprise Web", "format:check --projects=drive-web,identity-web,developer-web,enterprise-web"),
		packageScriptCheck("root-check-web-enterprise-lint", "check:web", "Root web check lints Enterprise Web", "lint --projects=drive-web,identity-web,developer-web,enterprise-web"),
		packageScriptCheck("root-check-web-enterprise-typecheck", "check:web", "Root web check typechecks Enterprise Web", "typecheck --projects=drive-web,identity-web,developer-web,enterprise-web"),
		packageScriptCheck("lint-web-enterprise", "lint:web", "Root lint:web includes Enterprise Web", "lint --projects=drive-web,identity-web,developer-web,enterprise-web"),
		textCheck("identity-router", sources.identityRouter, "Identity Web declares routed login/account journeys", "createRouter"),
		textCheck("identity-account-main", sources.identityAccountLayout, "Identity account shell exposes a main landmark", "<main"),
		textCheck("identity-critical-e2e", sources.identityCriticalE2e, "Identity Web has browser-level critical journey coverage", "critical identity journeys"),
		textCheck("identity-universal-login-test", sources.identityUniversalLoginTest, "Identity Web tests universal login API behavior", "Universal Login API helpers"),
		textCheck("identity-universal-login-hook-test", sources.identityUniversalLoginHookTest, "Identity Web tests universal login hook state", "useUniversalLogin"),
		textCheck("drive-router", sources.driveRouter, "Drive Web declares routed file journeys", "createRouter"),
		textCheck("drive-main", sources.driveLayout, "Drive Web shell exposes a main landmark", "<main"),
		textCheck("drive-share-actions", sources.driveSharedLinks, "Drive Web exposes share copy and revoke actions", "Revoquer le lien"),
		textCheck("drive-upload-test", sources.driveUploadTest, "Drive Web tests upload drop handling", "drop"),
		textCheck("drive-session-test", sources.driveSessionTest, "Drive Web tests session storage", "session"),
		textCheck("drive-workspace-test", sources.driveWorkspaceTest, "Drive Web tests workspace switching state", "workspace"),
		textCheck("developer-router", sources.developerRouter, "Developer Web declares routed console and portal journeys", "createRouter"),
		textCheck("developer-console-main", sources.developerShell, "Developer console shell exposes a main landmark", "<main"),
		textCheck("developer-portal-main", sources.developerPortalLayout, "Developer portal shell exposes a main landmark", "<main"),
		textCheck("developer-public-main", sources.developerPublicLayout, "Developer public shell exposes a main landmark", "<main"),
		textCheck("developer-permissions-test", sources.developerPermissionsTest, "Developer Web tests permission gating", "permission"),
		textCheck("developer-session-test", sources.developerSessionTest, "Developer Web tests session handling", "session"),
		textCheck("developer-webhooks-test", sources.developerWebhooksTest, "Developer Web tests webhook replay gating", "webhook"),
		textCheck("enterprise-router", sources.enterpriseRouter, "Enterprise Web declares routed admin journeys", "createRouter"),
		textCheck("enterprise-main", sources.enterpriseLayout, "Enterprise Web shell exposes a main landmark", "<main"),
		textCheck("enterprise-nav", sources.enterpriseSidebar, "Enterprise Web shell exposes navigation", "<nav"),
		textCheck("enterprise-permissions-test", sources.enterprisePermissionsTest, "Enterprise Web tests permission gating", "permission"),
		textCheck("enterprise-invites-test", sources.enterpriseInvitesTest, "Enterprise Web tests invitation behavior", "invite"),
		textCheck("cloud-console-boundary", sources.cloudConsoleReadme, "Cloud Console frontend boundary is documented", "Cloud Console"),
		textCheck("internal-admin-boundary", sources.internalAdminReadme, "Internal Admin frontend boundary is documented", "Internal Admin"),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in frontend source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "tools/migration/frontend-experience.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match frontend source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"pnpm check:web",
		"pnpm --dir apps/identity-web test:e2e:critical",
		"pnpm --dir apps/enterprise-web test",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "frontend-experience", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Frontend Experience Evidence",
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
		"- Every evidence row must be generated from the frontend source contract.",
		"- `passed` requires the configured file to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name the frontend checks required by the cutover gate.",
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
			? "Frontend repository evidence is covered for Identity, Drive, Developer, Enterprise, Cloud Console and Internal Admin boundaries. Production cutover still requires the G4 strict E2E and AA accessibility sign-off."
			: "Frontend repository evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-frontend-experience",
		"tools/migration/frontend-experience.mjs --write",
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
		command: "tools/migration/frontend-experience.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"pnpm check:web",
			"pnpm --dir apps/identity-web test:e2e:critical",
			"pnpm --dir apps/enterprise-web test",
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
	console.log(`Frontend experience evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/frontend-experience.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/frontend-experience.mjs --write`);
}

if (errors.length > 0) {
	console.error("Frontend experience evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Frontend experience evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
