#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/infra-deploy.generated.json";
const markdownPath = "docs/migration/infra-deploy.md";

const sources = {
	helmChart: "deploy/oss/helm/nvbes/Chart.yaml",
	helmValues: "deploy/oss/helm/nvbes/values.yaml",
	kustomize: "deploy/oss/kustomize/kustomization.yaml",
	compose: "deploy/oss/compose/compose.yaml",
	opentofuReadme: "deploy/oss/opentofu/README.md",
	stagingMain: "infrastructure/environments/staging/main.tf",
	stagingOutputs: "infrastructure/environments/staging/outputs.tf",
	stagingReadme: "infrastructure/environments/staging/README.md",
	productionReadme: "infrastructure/environments/production/README.md",
	productionAlloy: "infrastructure/environments/production/alloy.config.alloy",
	productionCompose: "infrastructure/environments/production/docker-compose.observability.yml",
	databaseModule: "infrastructure/modules/scaleway-v1/database.tf",
	storageModule: "infrastructure/modules/scaleway-v1/storage.tf",
	iamModule: "infrastructure/modules/scaleway-v1/iam.tf",
	testIntegration: "scripts/test-integration.sh",
	releaseGate: "scripts/release-gate.sh",
	migrateStaging: "scripts/migrate-staging.sh",
	smokeStaging: "scripts/smoke-staging.sh",
	backupRestoreManifest: "docs/migration/backup-restore-manifest.md",
	observabilityManifest: "docs/migration/observability-readiness.md",
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
		textCheck("helm-chart", sources.helmChart, "Helm chart is declared as an application chart", "type: application"),
		textCheck("helm-oci-registry", sources.helmValues, "Helm values pin the OCI image registry", "imageRegistry: ghcr.io/nvbes"),
		textCheck("helm-account-service-image", sources.helmValues, "Helm values include Account Service image", "repository: nvbes-account-service"),
		textCheck("helm-cloud-service-image", sources.helmValues, "Helm values include Cloud Service image", "repository: nvbes-cloud-service"),
		textCheck("kustomize-helm", sources.kustomize, "Kustomize references the Helm chart", "../helm/nvbes"),
		textCheck("compose-images", sources.compose, "Compose stack uses released OCI images", "ghcr.io/nvbes/nvbes-account-service"),
		textCheck("opentofu-boundary", sources.opentofuReadme, "OSS OpenTofu boundary is documented", "Cloud-specific managed deployment modules belong in `deploy/cloud`"),
		textCheck("staging-overlay", sources.stagingMain, "Staging OpenTofu overlay declares staging environment", 'environment = "staging"'),
		textCheck("staging-backup-retention", sources.stagingMain, "Staging overlay configures PostgreSQL backup retention", "postgres_backup_retention_days = 7"),
		textCheck("staging-secret-inventory", sources.stagingMain, "Staging overlay exposes required secret inventory", "secret_inventory = {"),
		textCheck("staging-cloudflare-tls", sources.stagingMain, "Staging edge enforces TLS settings", 'min_tls_version          = "1.3"'),
		textCheck("staging-outputs-secret-inventory", sources.stagingOutputs, "Staging outputs secret inventory without values", 'output "secret_inventory"'),
		textCheck("staging-readme-no-secrets", sources.stagingReadme, "Staging README forbids committing real tfvars secrets", "Do not commit `terraform.tfvars` or real secret values."),
		textCheck("postgres-encryption", sources.databaseModule, "PostgreSQL module enables encryption at rest", "encryption_at_rest        = true"),
		textCheck("postgres-backups", sources.databaseModule, "PostgreSQL module enables managed backups", "disable_backup            = false"),
		textCheck("object-versioning", sources.storageModule, "Object storage module enables bucket versioning", "versioning"),
		textCheck("object-lifecycle", sources.storageModule, "Object storage module defines lifecycle rules", "lifecycle_rule"),
		textCheck("runtime-secret-access", sources.iamModule, "Runtime IAM policy grants secret manager access", "SecretManagerSecretAccess"),
		textCheck("integration-tofu-validate", sources.testIntegration, "Integration script validates development and staging OpenTofu", "tofu -chdir=infrastructure/environments/staging validate"),
		textCheck("release-staging-smoke", sources.releaseGate, "Release gate runs staging smoke checks", "staging smoke gate"),
		textCheck("release-staging-e2e", sources.releaseGate, "Release gate runs staging critical E2E checks", "staging critical E2E gate"),
		textCheck("migration-backup-guard", sources.migrateStaging, "Staging migration requires backup or restore point confirmation", "fresh staging backup or restore point"),
		textCheck("smoke-staging", sources.smokeStaging, "Staging smoke wrapper runs smoke and E2E", "staging critical E2E"),
		textCheck("production-observability-doc", sources.productionReadme, "Production observability runbook documents Alloy architecture", "Alloy est obligatoire"),
		textCheck("production-secret-files", sources.productionReadme, "Production observability uses mounted secret files", "TOKEN_FILE"),
		textCheck("alloy-redaction", sources.productionAlloy, "Alloy pipeline redacts sensitive fields", "[FilteredSecret]"),
		textCheck("alloy-healthcheck", sources.productionCompose, "Production Alloy compose has a readiness healthcheck", "http://127.0.0.1:12345/-/ready"),
		textCheck("backup-restore-manifest", sources.backupRestoreManifest, "Backup/restore manifest enumerates restore evidence", "## Restore Evidence"),
		textCheck("observability-manifest", sources.observabilityManifest, "Observability manifest enumerates cutover evidence", "## Cutover Evidence"),
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
		if (!Object.values(sources).includes(check.path)) errors.push(`${check.id}: path is not in infra source contract`);
		if (!["passed", "failed"].includes(check.status)) errors.push(`${check.id}: unsupported status ${check.status}`);
		if (!check.pattern) errors.push(`${check.id}: pattern is required`);
	}
	const expectedSummary = summarize(report.checks);
	for (const [field, value] of Object.entries(expectedSummary)) {
		if (report.summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (report.generation?.command !== "node tools/migration/infra-deploy.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (!sameItems(report.generation?.sources, Object.values(sources))) {
		errors.push(`${outputPath}: generation.sources must match infra source contract`);
	}
	if (!sameItems(report.generation?.targeted_tests, [
		"pnpm check:supply-chain",
		"tofu -chdir=infrastructure/environments/development validate",
		"tofu -chdir=infrastructure/environments/staging validate",
		"pnpm check:migration-backup-restore",
		"pnpm check:migration-observability",
	])) {
		errors.push(`${outputPath}: generation.targeted_tests is invalid`);
	}
	for (const command of report.generation?.targeted_tests ?? []) {
		errors.push(...validateProofCommand({ id: "infra-deploy", proof: command }, packageScripts));
	}
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Infra Deploy Evidence",
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
		"- Every evidence row must be generated from the infra source contract.",
		"- `passed` requires the configured file to contain the expected pattern.",
		"- Summary counters must match evidence rows.",
		"- Targeted tests must name the infra checks required by the cutover gate.",
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
			? "Infra/deploy repository evidence is covered for OCI images, Helm, Kustomize, Compose, OpenTofu, secret inventory, backups and observability. Production cutover still requires the G5 strict staging rebuild, restore and rollback evidence."
			: "Infra/deploy evidence is blocked until failed checks pass.",
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-infra-deploy",
		"node tools/migration/infra-deploy.mjs --write",
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
		command: "node tools/migration/infra-deploy.mjs --write",
		sources: Object.values(sources),
		targeted_tests: [
			"pnpm check:supply-chain",
			"tofu -chdir=infrastructure/environments/development validate",
			"tofu -chdir=infrastructure/environments/staging validate",
			"pnpm check:migration-backup-restore",
			"pnpm check:migration-observability",
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
	console.log(`Infra/deploy evidence written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

for (const check of checks) {
	if (check.status === "failed") errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
	[outputPath, json],
	[markdownPath, markdown],
]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/infra-deploy.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/infra-deploy.mjs --write`);
}

if (errors.length > 0) {
	console.error("Infra/deploy evidence checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Infra/deploy evidence: ok (${summary.passed}/${summary.checks} checks passed)`);
