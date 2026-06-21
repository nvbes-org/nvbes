#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const outputPath = "docs/migration/codegen.generated.json";
const markdownPath = "docs/migration/codegen.md";
const errors = [];
const packageScripts = readPackageScripts(errors);

const controls = [
	{ id: "openapi", command: "pnpm check:contracts", evidence: "contracts/openapi/manifest.json" },
	{ id: "protobuf", command: "pnpm check:contracts", evidence: "contracts/protobuf/nvbes/platform/v1/common.proto" },
	{ id: "events", command: "pnpm check:contracts", evidence: "contracts/events/manifest.json" },
	{ id: "typescript-sdk", command: "pnpm check:codegen", evidence: "libs/ts/identity-sdk-core/src/types.gen.ts" },
	{ id: "rust-sdk", command: "pnpm check:codegen", evidence: "libs/rust/identity-sdk-backend/src/lib.rs" },
	{ id: "go-sdk", command: "pnpm check:codegen", evidence: "libs/go/identity-sdk/sdk.go" },
];

function buildLedger() {
	const entries = controls.map((control) => {
		const scriptName = control.command.replace(/^pnpm /, "");
		const active = Boolean(packageScripts[scriptName]) && existsSync(control.evidence);
		return { ...control, status: active ? "active" : "missing" };
	});
	return {
		schema_version: 1,
		generation: {
			command: "tools/migration/codegen.mjs --write",
			strict_cutover_command: "tools/migration/codegen.mjs --strict",
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
		"# Codegen and SDK Control Ledger",
		"",
		"## Status",
		"",
		`- entries: ${ledger.summary.entries}`,
		`- active: ${ledger.summary.active}`,
		`- pending: ${ledger.summary.pending}`,
		"",
		"## Rules",
		"",
		"- Control rows must match the static codegen contract.",
		"- Summary counters must match generated control rows.",
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
	lines.push("", "## Regeneration", "", "```bash", "pnpm check:migration-codegen", "tools/migration/codegen.mjs --write", "```", "");
	return lines.join("\n");
}

function validate(ledger) {
	const expectedById = new Map(controls.map((control) => [control.id, control]));
	const seen = new Set();
	if (ledger.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (ledger.generation?.command !== "tools/migration/codegen.mjs --write") errors.push(`${outputPath}: generation.command is invalid`);
	if (ledger.generation?.strict_cutover_command !== "tools/migration/codegen.mjs --strict") errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	if (!Array.isArray(ledger.entries) || ledger.entries.length !== controls.length) {
		errors.push(`${outputPath}: entries must include every codegen control`);
	}
	validateSummary(ledger);
	for (const entry of ledger.entries ?? []) {
		if (seen.has(entry.id)) errors.push(`${outputPath}: duplicate control ${entry.id}`);
		seen.add(entry.id);
		const expected = expectedById.get(entry.id);
		if (!expected) {
			errors.push(`${outputPath}: unexpected control ${entry.id}`);
			continue;
		}
		if (entry.command !== expected.command) errors.push(`${entry.id}: command must match static control`);
		errors.push(...validateProofCommand({ id: entry.id, proof: entry.command }, packageScripts));
		if (entry.evidence !== expected.evidence) errors.push(`${entry.id}: evidence must match static control`);
		if (!["active", "missing"].includes(entry.status)) errors.push(`${entry.id}: unsupported status ${entry.status}`);
		if (entry.status === "active" && !existsSync(entry.evidence)) errors.push(`${entry.id}: evidence file is missing`);
		if (strict && entry.status !== "active") errors.push(`${entry.id}: control is not active`);
	}
	for (const id of expectedById.keys()) {
		if (!seen.has(id)) errors.push(`${outputPath}: missing control ${id}`);
	}
}

function validateSummary(ledger) {
	const entries = Array.isArray(ledger.entries) ? ledger.entries : [];
	const active = entries.filter((entry) => entry.status === "active").length;
	if (ledger.summary.entries !== entries.length) errors.push(`${outputPath}: summary entries must match control rows`);
	if (ledger.summary.active !== active) errors.push(`${outputPath}: summary active must match control rows`);
	if (ledger.summary.pending !== entries.length - active) {
		errors.push(`${outputPath}: summary pending must match control rows`);
	}
}

const ledger = buildLedger();
const json = serializeJson(ledger);
const markdown = serializeMarkdown(ledger);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration codegen ledger written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(ledger);
for (const [path, expected] of [[outputPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/codegen.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/codegen.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration codegen checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration codegen: ok (${ledger.summary.active}/${ledger.summary.entries} controls active)`);
