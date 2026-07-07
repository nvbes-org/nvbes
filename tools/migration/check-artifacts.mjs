#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { required } from "./check-artifacts.catalog.mjs";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const forbiddenMarkers = [/\bTODO\b/i, /\bTBD\b/i, /\bFIXME\b/i, /a completer/i, /a compléter/i];
const requiredArtifactPaths = new Set(required.map((artifact) => artifact.path));
const ignoredPnpmTokens = new Set(["checks"]);
const commandCoverage = [
	["docs/migration/snapshot-manifest.md", ["pnpm check:migration-snapshots -- --strict"]],
	["docs/migration/release-freeze-manifest.md", ["pnpm check:migration-release-freeze -- --strict"]],
	["docs/migration/backup-restore-manifest.md", ["pnpm check:migration-backup-restore -- --strict"]],
	["docs/migration/observability-readiness.md", ["pnpm check:migration-observability -- --strict"]],
	["docs/migration/smoke-test-manifest.md", ["pnpm check:migration-smoke-tests -- --strict"]],
	["docs/migration/rehearsal-ledger.md", ["pnpm check:migration-rehearsals -- --strict"]],
	["docs/migration/owner-signoff-matrix.md", ["pnpm check:migration-owner-signoffs -- --strict"]],
	["docs/migration/cutover-checklist.md", ["pnpm check:migration-cutover-checklist -- --strict"]],
	["docs/migration/communication-plan.md", ["pnpm check:migration-communication -- --strict"]],
	["docs/migration/rollback-report.md", ["pnpm check:migration-rollback-report -- --strict"]],
	["docs/migration/rejects.md", ["pnpm check:migration-rejects -- --strict"]],
	["docs/migration/cutover-journal.md", ["pnpm check:migration-cutover-journal -- --strict"]],
	["docs/migration/decommission-manifest.md", ["pnpm check:migration-decommission -- --strict"]],
	["docs/migration/v2-debt-register.md", ["pnpm check:migration-v2-debt -- --strict"]],
	["docs/migration/live-evidence.md", ["pnpm check:migration-live-evidence-schema", "pnpm check:migration-live-evidence-rules"]],
	[
		"docs/migration/live-evidence-instances.md",
		["pnpm check:migration-live-evidence-instances", "pnpm check:migration-live-evidence-instances -- --strict"],
	],
];
const generatedArtifactChecks = [
	["docs/migration/readiness-report.generated.json", "pnpm check:migration-readiness-report", "node tools/migration/readiness-report.mjs --write"],
	["docs/migration/completion-audit.generated.json", "pnpm check:migration-completion-audit", "node tools/migration/completion-audit.mjs --write"],
	["docs/migration/execution-backlog.generated.json", "pnpm check:migration-execution-backlog", "node tools/migration/execution-backlog.mjs --write"],
	["tools/migration/status-consistency.mjs", "pnpm check:migration-status-consistency"],
	["docs/migration/cutover-evidence-packet.generated.json", "pnpm check:migration-cutover-evidence-packet", "node tools/migration/cutover-evidence-packet.mjs --write"],
	["docs/migration/live-evidence-instances.generated.json", "pnpm check:migration-live-evidence-instances", "node tools/migration/live-evidence-instances.mjs --write"],
];
const errors = [];
const packageScripts = readPackageScripts(errors);

for (const artifact of required) {
	validateArtifact(artifact);
}
validateGeneratedArtifactChecks();
validateCommandCoverage();
validateDocumentedRunnableCommands();
validateDocumentedPnpmScripts();
validateDocumentedMigrationTools();

function validateArtifact(artifact) {
	if (!existsSync(artifact.path)) {
		errors.push(`${artifact.path}: missing required migration artifact`);
		return;
	}

	const content = readFileSync(artifact.path, "utf8");
	for (const heading of artifact.headings) {
		if (!content.includes(heading)) {
			errors.push(`${artifact.path}: missing heading "${heading}"`);
		}
	}

	for (const marker of forbiddenMarkers) {
		if (marker.test(content)) {
			errors.push(`${artifact.path}: unresolved marker ${marker}`);
		}
	}
}

function validateGeneratedArtifactChecks() {
	for (const [path, command, generationCommand] of generatedArtifactChecks) {
		if (!existsSync(path)) {
			errors.push(`${path}: generated artifact proof target is missing`);
			continue;
		}
		if (path.startsWith("docs/migration/") && !requiredArtifactPaths.has(path)) errors.push(`${path}: generated artifact is not listed in required artifacts`);
		if (generationCommand && readGeneratedCommand(path) !== generationCommand) errors.push(`${path}: generation.command must be ${generationCommand}`);
		errors.push(...validateProofCommand({ id: path, proof: command }, packageScripts));
	}
}

function readGeneratedCommand(path) {
	try {
		return JSON.parse(readFileSync(path, "utf8")).generation?.command;
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
	}
}

function validateCommandCoverage() {
	const seenPaths = new Set();
	for (const [path, commands] of commandCoverage) {
		if (seenPaths.has(path)) {
			errors.push(`${path}: duplicate command coverage entry`);
		}
		seenPaths.add(path);
		if (!requiredArtifactPaths.has(path)) {
			errors.push(`${path}: command coverage artifact is not listed in required artifacts`);
		}
		if (!existsSync(path)) {
			errors.push(`${path}: missing command coverage artifact`);
			continue;
		}
		const content = readFileSync(path, "utf8");
		if (!content.includes("## Verification")) {
			errors.push(`${path}: missing Verification section`);
		}
		const seenCommands = new Set();
		for (const command of commands) {
			if (seenCommands.has(command)) {
				errors.push(`${path}: duplicate verification command "${command}"`);
			}
			seenCommands.add(command);
			if (!content.includes(command)) {
				errors.push(`${path}: missing verification command "${command}"`);
			}
			errors.push(...validateProofCommand({ id: path, proof: command }, packageScripts));
		}
	}
}

function validateDocumentedPnpmScripts() {
	for (const artifact of required) {
		if (!existsSync(artifact.path)) continue;
		const content = readFileSync(artifact.path, "utf8");
		for (const match of content.matchAll(/pnpm\s+(?!-)([a-z0-9:][a-z0-9:-]*)/g)) {
			if (ignoredPnpmTokens.has(match[1])) continue;
			if (!packageScripts[match[1]]) {
				errors.push(`${artifact.path}: documented script ${match[1]} is missing from package.json`);
			}
		}
	}
}

function validateDocumentedRunnableCommands() {
	for (const artifact of required) {
		if (!existsSync(artifact.path)) continue;
		const content = readFileSync(artifact.path, "utf8");
		for (const line of content.split("\n")) {
			const command = line.trim();
			if (!isRunnableCommand(command)) continue;
			errors.push(...validateProofCommand({ id: artifact.path, proof: command }, packageScripts));
		}
	}
}

function isRunnableCommand(command) {
	return /^(pnpm|node tools\/migration\/|tools\/migration\/|cargo |go |tofu |bash scripts\/)/.test(command);
}

function validateDocumentedMigrationTools() {
	for (const artifact of required) {
		if (!existsSync(artifact.path)) continue;
		const content = readFileSync(artifact.path, "utf8");
		for (const match of content.matchAll(/tools\/migration\/[a-z0-9.-]+/g)) {
			if (!match[0].endsWith(".mjs")) {
				errors.push(`${artifact.path}: documented migration tool ${match[0]} must include .mjs`);
				continue;
			}
			if (!existsSync(match[0])) {
				errors.push(`${artifact.path}: documented migration tool ${match[0]} is missing`);
			}
		}
	}
}

if (errors.length > 0) {
	console.error("Migration artifact checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration artifacts: ok (${required.length} files checked)`);
