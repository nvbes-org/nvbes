#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { readPackageScripts } from "./execution-backlog.proof.mjs";

const errors = [];
const scripts = readPackageScripts(errors);
const rootCheck = scripts.check ?? "";
const runbook = readFileSync("docs/migration/nvbes-big-bang-migration-runbook.md", "utf8");
const migrationGenerate = readFileSync("tools/migration/generate.mjs", "utf8");
const migrationToolFiles = readdirSync("tools/migration").filter((name) => name.endsWith(".mjs"));

const allowedUnlistedRunbookScripts = new Set([
	"check:migration-artifacts",
	"check:migration-postcutover",
]);

const excludedMigrationScripts = new Set([
	"check:migration-precutover",
	"check:migration-postcutover",
]);

const requiredGateTools = [
	"tools/migration/repository-controls.mjs",
	"tools/migration/precutover-gate.mjs",
	"tools/migration/postcutover-gate.mjs",
	"tools/migration/check-artifacts.catalog.mjs",
	"tools/migration/check-artifacts.catalog.core.mjs",
	"tools/migration/check-artifacts.catalog.evidence.mjs",
	"tools/migration/check-artifacts.catalog.operations.mjs",
	"tools/migration/domain-ledger.evidence.mjs",
	"tools/migration/domain-dod.evidence.mjs",
	"tools/migration/cutover-evidence-packet.requirements.mjs",
	"tools/migration/live-evidence-instances.validation.mjs",
	"tools/migration/execution-backlog.proof.mjs",
	"tools/migration/cli-options.mjs",
	"tools/migration/live-evidence-prepare.mjs",
	"tools/migration/decision-map.validation.mjs",
	"tools/migration/inventory.validation.mjs",
];

const requiredMigrationScripts = [
	"check:migration-cli-options",
	"check:migration-blueprint",
	"check:migration-runbook",
	"check:migration-tooling",
	"check:migration-repository-controls",
	"check:migration-cutover-gates",
	"check:migration-inventory",
	"check:migration-data-map",
	"check:migration-secret-map",
	"check:migration-job-map",
	"check:migration-resource-map",
	"check:migration-readiness-report",
	"check:migration-completion-audit",
	"check:migration-execution-backlog",
	"check:migration-status-consistency",
	"check:migration-cutover-evidence-packet",
	"check:migration-live-evidence-schema",
	"check:migration-live-evidence-rules",
	"check:migration-live-evidence-instances",
	"check:migration-live-evidence-commands",
	"check:migration-live-evidence-prepare",
	"check:migration-target-structure",
	"check:migration-codegen",
	"check:migration-supply-chain",
	"check:migration-runtime-foundation",
	"check:migration-platform-primitives",
	"check:migration-data-migration-pipeline",
	"check:migration-identity-register",
	"check:migration-identity-login-session",
	"check:migration-identity-mfa-webauthn",
	"check:migration-workspace-membership-roles",
	"check:migration-workspace-last-owner",
	"check:migration-drive-upload-download",
	"check:migration-drive-share-revoke",
	"check:migration-drive-quotas",
	"check:migration-audit-append-only",
	"check:migration-privacy-export-delete",
	"check:migration-billing-entitlements",
	"check:migration-billing-webhook-idempotency",
	"check:migration-developer-oauth-tokens",
	"check:migration-developer-signed-webhooks",
	"check:migration-cloud-provisioning",
	"check:migration-frontend-experience",
	"check:migration-infra-deploy",
	"check:migration-phase-ledger",
	"check:migration-domain-ledger",
	"check:migration-domain-dod",
	"check:migration-risk-register",
	"check:migration-gate-evidence",
	"check:migration-parity",
	"check:migration-owner-signoffs",
	"check:migration-cutover-checklist",
	"check:migration-rehearsals",
	"check:migration-snapshots",
	"check:migration-release-freeze",
	"check:migration-backup-restore",
	"check:migration-communication",
	"check:migration-observability",
	"check:migration-smoke-tests",
	"check:migration-rollback-report",
	"check:migration-rejects",
	"check:migration-reconciliation-report",
	"check:migration-cutover-journal",
	"check:migration-post-migration-audit",
	"check:migration-decommission",
	"check:migration-v2-debt",
	"check:migration-reconciliation-template",
	"check:migration-artifacts",
];

const requiredRepositoryScripts = [
	"verify",
	"nx:affected",
	"check:web",
	"lint:web",
	"check:api",
	"check:go",
	"check:python",
	"check:contracts",
	"check:codegen",
	"check:product-boundaries",
	"check:secrets",
	"check:supply-chain",
	"check:oss-boundaries",
	"migration:generate",
];

const requiredRootRepositoryScripts = [
	"check:web",
	"check:api",
	"check:go",
	"check:python",
	"check:contracts",
	"check:product-boundaries",
	"check:secrets",
	"check:oss-boundaries",
];

const requiredRunbookRepositoryCommands = [
	"pnpm verify",
	"pnpm nx:affected",
	"cargo check --workspace",
	"pnpm check:go",
	"pnpm check:python",
	"pnpm check:web",
	"pnpm lint:web",
	"pnpm check:contracts",
	"pnpm check:codegen",
	"pnpm check:product-boundaries",
	"pnpm check:secrets",
	"pnpm check:supply-chain",
	"pnpm migration:generate",
];

const requiredGeneratedReports = [
	"tools/migration/readiness-report.mjs",
	"tools/migration/completion-audit.mjs",
	"tools/migration/execution-backlog.mjs",
	"tools/migration/cutover-evidence-packet.mjs",
	"tools/migration/live-evidence-instances.mjs",
];
const generatorOrder = [
	"tools/migration/readiness-report.mjs",
	"tools/migration/cutover-evidence-packet.mjs",
	"tools/migration/live-evidence-instances.mjs",
	"tools/migration/completion-audit.mjs",
	"tools/migration/execution-backlog.mjs",
];

const migrationScripts = Object.entries(scripts).filter(([name]) =>
	name.startsWith("check:migration-"),
);

for (const name of requiredRepositoryScripts) {
	if (!scripts[name]) errors.push(`${name}: package script is required`);
}
if (scripts["migration:generate"] !== "node tools/migration/generate.mjs") {
	errors.push("migration:generate must run node tools/migration/generate.mjs");
}

for (const name of requiredRootRepositoryScripts) {
	if (!rootCheck.includes(`pnpm ${name}`)) errors.push(`${name}: missing from root check`);
}

for (const name of requiredRepositoryScripts) {
	const nodeScript = scripts[name]?.match(/^node (tools\/migration\/[^\s]+)/)?.[1];
	if (nodeScript && !existsSync(nodeScript)) {
		errors.push(`${name}: missing tool file ${nodeScript}`);
	}
}

for (const command of requiredRunbookRepositoryCommands) {
	if (!runbook.includes(command)) errors.push(`${command}: missing from runbook validation commands`);
}

for (const generator of requiredGeneratedReports) {
	if (!migrationGenerate.includes(`"${generator}", ["--write"]`)) {
		errors.push(`${generator}: missing from migration generator write pipeline`);
	}
}
validateGeneratorOrder();

for (const [name, command] of migrationScripts) {
	if (!rootCheck.includes(`pnpm ${name}`) && !excludedMigrationScripts.has(name)) {
		errors.push(`${name}: missing from root check`);
	}

	if (!runbook.includes(`pnpm ${name}`) && !allowedUnlistedRunbookScripts.has(name)) {
		errors.push(`${name}: missing from runbook validation commands`);
	}

	const nodeScript = command.match(/^node (tools\/migration\/[^\s]+)/)?.[1];
	if (nodeScript && !existsSync(nodeScript)) {
		errors.push(`${name}: missing tool file ${nodeScript}`);
	}
}

for (const gateTool of requiredGateTools) {
	if (!existsSync(gateTool)) errors.push(`${gateTool}: missing gate tool`);
}

for (const script of requiredMigrationScripts) {
	if (!scripts[script]) errors.push(`${script}: package script is required`);
}

for (const gateScript of excludedMigrationScripts) {
	if (!scripts[gateScript]) errors.push(`${gateScript}: package script is required`);
}

const duplicateScripts = migrationScripts
	.map(([name]) => name)
	.filter((name, index, names) => names.indexOf(name) !== index);
for (const script of duplicateScripts) {
	errors.push(`${script}: duplicate migration script`);
}

validateNoDirectPackageJsonParse();
validateMigrationToolFileSizes();

function validateGeneratorOrder() {
	for (let index = 1; index < generatorOrder.length; index += 1) {
		const previous = migrationGenerate.indexOf(`"${generatorOrder[index - 1]}"`);
		const current = migrationGenerate.indexOf(`"${generatorOrder[index]}"`);
		if (previous === -1 || current === -1 || previous > current) errors.push(`${generatorOrder[index]}: migration generator order is invalid`);
	}
}

function validateNoDirectPackageJsonParse() {
	const forbidden = /JSON\.parse\(readFileSync\("package\.json"/;
	for (const file of migrationToolFiles) {
		const path = `tools/migration/${file}`;
		if (forbidden.test(readFileSync(path, "utf8"))) {
			errors.push(`${path}: use readPackageScripts(errors) instead of direct package.json parse`);
		}
	}
}

function validateMigrationToolFileSizes() {
	for (const file of migrationToolFiles) {
		const path = `tools/migration/${file}`;
		const lines = readFileSync(path, "utf8").split(/\n/).length;
		if (lines > 300) errors.push(`${path}: ${lines} lines exceeds 300 line cap`);
	}
}

if (errors.length > 0) {
	console.error("Migration tooling checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Migration tooling: ok (${migrationScripts.length} migration scripts, ${requiredRepositoryScripts.length} repository scripts)`,
);
