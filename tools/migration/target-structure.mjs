#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const sourcePath = "docs/blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md";
const outputPath = "docs/migration/target-structure.generated.json";
const markdownPath = "docs/migration/target-structure.md";
const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const errors = [];

function read(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return "";
	}
	return readFileSync(path, "utf8");
}

function parseTargetTree(markdown) {
	const start = markdown.indexOf("## Structure Monorepo Cible");
	if (start === -1) return [];
	const codeStart = markdown.indexOf("```text", start);
	const codeEnd = markdown.indexOf("```", codeStart + 1);
	if (codeStart === -1 || codeEnd === -1) return [];
	const stack = [];
	const paths = [];
	for (const line of markdown.slice(codeStart, codeEnd).split("\n")) {
		const match = line.match(/^(\s*)([^#\s][^#]*?\/)/);
		if (!match) continue;
		const depth = Math.floor(match[1].length / 2);
		const segment = match[2].trim().replace(/\/$/, "");
		if (segment === "nvbes") continue;
		stack[depth] = segment;
		stack.length = depth + 1;
		paths.push(stack.slice(1).join("/"));
	}
	return [...new Set(paths.filter(Boolean))].sort();
}

function buildLedger() {
	const expectedPaths = parseTargetTree(read(sourcePath));
	const entries = expectedPaths.map((path) => ({
		path,
		status: existsSync(path) ? "present" : "missing",
		proof: path,
	}));
	const summary = {
		entries: entries.length,
		present: entries.filter((entry) => entry.status === "present").length,
		missing: entries.filter((entry) => entry.status === "missing").length,
		pending: entries.filter((entry) => entry.status !== "present").length,
	};
	return {
		schema_version: 1,
		generation: {
			command: "tools/migration/target-structure.mjs --write",
			source: sourcePath,
			strict_cutover_command: "tools/migration/target-structure.mjs --strict",
		},
		summary,
		entries,
	};
}

function serializeJson(ledger) {
	return `${JSON.stringify(ledger, null, 2)}\n`;
}

function serializeMarkdown(ledger) {
	const lines = [
		"# Target Monorepo Structure Ledger",
		"",
		"## Status",
		"",
		`- entries: ${ledger.summary.entries}`,
		`- present: ${ledger.summary.present}`,
		`- missing: ${ledger.summary.missing}`,
		"",
		"## Rules",
		"",
		"- Path rows must match the blueprint target monorepo tree.",
		"- Summary counters must match generated path rows.",
		"- Generation provenance must identify source, write command and strict cutover command.",
		"",
		"## Paths",
		"",
		"| Path | Status | Proof |",
		"|---|---:|---|",
	];
	for (const entry of ledger.entries) {
		lines.push(`| \`${entry.path}\` | ${entry.status} | \`${entry.proof}\` |`);
	}
	lines.push("", "## Regeneration", "", "```bash", "pnpm check:migration-target-structure", "tools/migration/target-structure.mjs --write", "```", "");
	return lines.join("\n");
}

function validate(ledger) {
	const expectedPaths = new Set(parseTargetTree(read(sourcePath)));
	const seen = new Set();
	if (ledger.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (ledger.generation?.command !== "tools/migration/target-structure.mjs --write") errors.push(`${outputPath}: generation.command is invalid`);
	if (ledger.generation?.source !== sourcePath) errors.push(`${outputPath}: generation.source is invalid`);
	if (ledger.generation?.strict_cutover_command !== "tools/migration/target-structure.mjs --strict") errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	if (!Array.isArray(ledger.entries) || ledger.entries.length === 0) {
		errors.push(`${outputPath}: entries must include target monorepo paths`);
	}
	validateSummary(ledger);
	for (const entry of ledger.entries ?? []) {
		if (!entry.path) errors.push(`${outputPath}: path is required`);
		if (seen.has(entry.path)) errors.push(`${outputPath}: duplicate path ${entry.path}`);
		seen.add(entry.path);
		if (!expectedPaths.has(entry.path)) errors.push(`${outputPath}: stale path ${entry.path}`);
		if (!["present", "missing"].includes(entry.status)) errors.push(`${entry.path}: unsupported status ${entry.status}`);
		if (entry.proof !== entry.path) errors.push(`${entry.path}: proof must match path`);
		if (entry.status === "present" && !existsSync(entry.path)) errors.push(`${entry.path}: present path is missing`);
		if (entry.status === "missing" && existsSync(entry.path)) errors.push(`${entry.path}: missing path exists`);
		if (strict && entry.status !== "present") errors.push(`${entry.path}: target path missing`);
	}
	for (const path of expectedPaths) {
		if (!seen.has(path)) errors.push(`${outputPath}: missing path ${path}`);
	}
}

function validateSummary(ledger) {
	const entries = Array.isArray(ledger.entries) ? ledger.entries : [];
	const present = entries.filter((entry) => entry.status === "present").length;
	const missing = entries.filter((entry) => entry.status === "missing").length;
	if (ledger.summary.entries !== entries.length) errors.push(`${outputPath}: summary entries must match path rows`);
	if (ledger.summary.present !== present) errors.push(`${outputPath}: summary present must match path rows`);
	if (ledger.summary.missing !== missing) errors.push(`${outputPath}: summary missing must match path rows`);
	if (ledger.summary.pending !== missing) errors.push(`${outputPath}: summary pending must match missing path rows`);
}

const ledger = buildLedger();
const json = serializeJson(ledger);
const markdown = serializeMarkdown(ledger);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration target structure ledger written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(ledger);

for (const [path, expected] of [[outputPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/migration/target-structure.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/migration/target-structure.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration target structure checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration target structure: ok (${ledger.summary.present}/${ledger.summary.entries} paths present)`);
