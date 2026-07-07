#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const outputPath = "docs/migration/supply-chain.generated.json";
const markdownPath = "docs/migration/supply-chain.md";
const errors = [];
const packageScripts = readPackageScripts(errors);

const controls = [
	{ id: "secrets", command: "pnpm check:secrets", evidence: "tools/security/scan-secrets.mjs" },
	{ id: "licenses", command: "pnpm check:oss-licenses", evidence: "scripts/check-oss-licenses.mjs" },
	{ id: "dependencies", command: "pnpm check:dependencies", evidence: "tools/security/check-dependencies.mjs" },
	{ id: "sbom", command: "pnpm check:sbom", evidence: "docs/migration/sbom.generated.json" },
	{ id: "containers", command: "pnpm check:containers", evidence: "tools/security/check-containers.mjs" },
];

function commandConfigured(command, packageScripts) {
	if (!command.startsWith("pnpm ")) return false;
	return Boolean(packageScripts[command.slice("pnpm ".length)]);
}

function buildLedger() {
	const entries = controls.map((control) => {
		const present = commandConfigured(control.command, packageScripts) && existsSync(control.evidence);
		return { ...control, status: present ? "active" : "missing" };
	});
	return {
		schema_version: 1,
		generation: {
			command: "node tools/migration/supply-chain.mjs --write",
			strict_cutover_command: "node tools/migration/supply-chain.mjs --strict",
		},
		summary: {
			entries: entries.length,
			active: entries.filter((entry) => entry.status === "active").length,
			pending: entries.filter((entry) => entry.status !== "active").length,
		},
		entries,
	};
}

function serializeJson(ledger) {
	return `${JSON.stringify(ledger, null, 2)}\n`;
}

function serializeMarkdown(ledger) {
	const lines = [
		"# Supply Chain Control Ledger",
		"",
		"## Status",
		"",
		`- entries: ${ledger.summary.entries}`,
		`- active: ${ledger.summary.active}`,
		`- pending: ${ledger.summary.pending}`,
		"",
		"## Rules",
		"",
		"- Every control row must match the static supply-chain contract.",
		"- `active` requires the package script and evidence path to exist.",
		"- Summary counters must match generated rows.",
		"- Strict cutover requires every control to be active.",
		"- Generation provenance must identify write and strict cutover commands.",
		"",
		"## Controls",
		"",
		"| Control | Status | Command | Evidence |",
		"|---|---:|---|---|",
	];
	for (const entry of ledger.entries) {
		lines.push(`| ${entry.id} | ${entry.status} | \`${entry.command}\` | \`${entry.evidence}\` |`);
	}
	lines.push("", "## Regeneration", "", "```bash", "pnpm check:migration-supply-chain", "node tools/migration/supply-chain.mjs --write", "```", "");
	return lines.join("\n");
}

function validate(ledger) {
	const byId = new Map(controls.map((control) => [control.id, control]));
	const seen = new Set();
	if (ledger.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (!Array.isArray(ledger.entries) || ledger.entries.length !== controls.length) {
		errors.push(`${outputPath}: entries must include every supply-chain control`);
	}
	for (const entry of ledger.entries ?? []) {
		if (!entry.id) {
			errors.push(`${outputPath}: control id is required`);
			continue;
		}
		if (seen.has(entry.id)) errors.push(`${outputPath}: duplicate control ${entry.id}`);
		seen.add(entry.id);
		const expected = byId.get(entry.id);
		if (!expected) {
			errors.push(`${outputPath}: stale control ${entry.id}`);
			continue;
		}
		if (entry.command !== expected.command) errors.push(`${entry.id}: command must be ${expected.command}`);
		errors.push(...validateProofCommand({ id: entry.id, proof: entry.command }, packageScripts));
		if (entry.evidence !== expected.evidence) errors.push(`${entry.id}: evidence must be ${expected.evidence}`);
		if (!["active", "missing"].includes(entry.status)) errors.push(`${entry.id}: unsupported status ${entry.status}`);
		const configured = commandConfigured(expected.command, packageScripts);
		const evidencePresent = existsSync(expected.evidence);
		const expectedStatus = configured && evidencePresent ? "active" : "missing";
		if (entry.status !== expectedStatus) errors.push(`${entry.id}: status must be ${expectedStatus}`);
		if (entry.status === "active" && !configured) errors.push(`${entry.id}: package script missing for ${expected.command}`);
		if (entry.status === "active" && !evidencePresent) errors.push(`${entry.id}: evidence path missing: ${expected.evidence}`);
		if (strict && entry.status !== "active") errors.push(`${entry.id}: control is not active`);
	}
	for (const control of controls) {
		if (!seen.has(control.id)) errors.push(`${outputPath}: missing control ${control.id}`);
	}
	const summary = ledger.summary ?? {};
	const active = (ledger.entries ?? []).filter((entry) => entry.status === "active").length;
	const pending = (ledger.entries ?? []).filter((entry) => entry.status !== "active").length;
	if (summary.entries !== controls.length) errors.push(`${outputPath}: summary.entries must be ${controls.length}`);
	if (summary.active !== active) errors.push(`${outputPath}: summary.active must be ${active}`);
	if (summary.pending !== pending) errors.push(`${outputPath}: summary.pending must be ${pending}`);
	if (strict && pending !== 0) errors.push(`${outputPath}: strict cutover requires zero pending controls`);
	if (ledger.generation?.command !== "node tools/migration/supply-chain.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (ledger.generation?.strict_cutover_command !== "node tools/migration/supply-chain.mjs --strict") {
		errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	}
}

const ledger = buildLedger();
const json = serializeJson(ledger);
const markdown = serializeMarkdown(ledger);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration supply chain ledger written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(ledger);
for (const [path, expected] of [[outputPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/supply-chain.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/supply-chain.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration supply-chain checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration supply chain: ok (${ledger.summary.active}/${ledger.summary.entries} controls active)`);
