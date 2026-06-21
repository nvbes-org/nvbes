#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/cloud-provisioning.generated.json";
const markdownPath = "docs/migration/cloud-provisioning.md";
const sources = {
	provisioning: "libs/go/provisioning/provisioning.go",
	provisioningTests: "libs/go/provisioning/provisioning_test.go",
	provisioningReadme: "libs/go/provisioning/README.md",
	controlPlane: "libs/go/control-plane/foundation.go",
	controlPlaneTests: "libs/go/control-plane/foundation_test.go",
	cloudControlApi: "apps/cloud-control-api/README.md",
	cloudConsole: "apps/cloud-console/README.md",
	cloudApps: "apps/cloud/README.md",
	cloudDeploy: "deploy/cloud/README.md",
	cloudDocs: "docs/cloud/README.md",
	targetStructure: "docs/migration/target-structure.generated.json",
	ossExport: "tools/oss-export/manifest.json",
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
		textCheck("request-model", sources.provisioning, "Cloud provisioning exposes a request model", "type WorkspaceProvisioningRequest struct"),
		textCheck("plan-model", sources.provisioning, "Cloud provisioning exposes a plan model", "type ProvisioningPlan struct"),
		textCheck("deterministic-idempotency", sources.provisioning, "Provisioning plans use deterministic idempotency keys", "func idempotencyKey"),
		textCheck("required-audit", sources.provisioning, "Provisioning plans require audit event names", "RequiredAuditType: \"cloud.provisioning.plan_created\""),
		textCheck("ordered-steps", sources.provisioning, "Provisioning plan contains ordered cloud resource steps", "object-storage-bucket"),
		textCheck("sla-classification", sources.provisioning, "Provisioning class maps to SLA", "func slaFor"),
		textCheck("rollback-event", sources.provisioning, "Cloud rollback event name is declared", "cloud.provisioning.rollback_requested"),
		textCheck("deterministic-test", sources.provisioningTests, "Provisioning idempotency and audit behavior are tested", "TestBuildProvisioningPlanIsDeterministicAndAudited"),
		textCheck("validation-test", sources.provisioningTests, "Provisioning validation behavior is tested", "TestBuildProvisioningPlanRejectsInvalidRequests"),
		textCheck("events-test", sources.provisioningTests, "Cloud audit event names are tested", "TestAuditEventTypesAreVersionableCloudEvents"),
		textCheck("provider-neutral-doc", sources.provisioningReadme, "Provisioning package is provider-neutral by contract", "Provider-specific adapters must live outside this package."),
		textCheck("control-plane-health", sources.controlPlane, "Cloud control-plane runtime has health surface", "func HealthStatus() string"),
		textCheck("control-plane-test", sources.controlPlaneTests, "Cloud control-plane health is tested", "TestHealthStatus"),
		textCheck("control-api-boundary", sources.cloudControlApi, "Cloud Control API boundary is documented", "plan/apply orchestration"),
		textCheck("console-boundary", sources.cloudConsole, "Cloud Console boundary is documented", "provisioning plans"),
		textCheck("cloud-apps-boundary", sources.cloudApps, "Cloud app boundary exists", "Cloud control-plane applications"),
		textCheck("cloud-deploy-boundary", sources.cloudDeploy, "Cloud deployment boundary exists", "provider-specific infrastructure"),
		textCheck("cloud-docs-boundary", sources.cloudDocs, "Cloud docs boundary exists", "managed hosting, billing, regions, support, SLA and cloud operations"),
		textCheck("target-structure-cloud-apps", sources.targetStructure, "Target structure includes Cloud app boundary", "\"apps/cloud\""),
		textCheck("target-structure-provisioning", sources.targetStructure, "Target structure includes Go provisioning", "\"libs/go/provisioning\""),
		textCheck("oss-export-excludes-cloud", sources.ossExport, "OSS export manifest excludes Cloud-only paths", "\"exclude\""),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in cloud provisioning source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "tools/migration/cloud-provisioning.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match cloud provisioning source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, ["go test ./libs/go/provisioning ./libs/go/control-plane"])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "cloud-provisioning", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Cloud Provisioning Evidence",
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
		"- Every evidence row must be generated from the cloud provisioning source contract.",
		"- `passed` requires the configured Go, README, target-structure, or OSS export source to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name the Go provisioning and control-plane packages required by parity.",
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
			? "Cloud provisioning repository evidence is covered. Production cutover still requires environment-specific apply, rollback, SLA and support evidence."
			: "Cloud provisioning evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-cloud-provisioning",
		"tools/migration/cloud-provisioning.mjs --write",
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
		command: "tools/migration/cloud-provisioning.mjs --write",
		sources: Object.values(sources),
		targeted_tests: ["go test ./libs/go/provisioning ./libs/go/control-plane"],
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
	console.log(`Cloud provisioning evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/cloud-provisioning.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/cloud-provisioning.mjs --write`);
}

if (errors.length > 0) {
	console.error("Cloud provisioning evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Cloud provisioning evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
