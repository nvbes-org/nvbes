#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts } from "./execution-backlog.proof.mjs";

const outputPath = "docs/migration/runtime-foundation.generated.json";
const markdownPath = "docs/migration/runtime-foundation.md";
const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const errors = [];

const required = [
	{
		runtime: "Rust",
		scope: "backend core and product/platform crates",
		required_paths: ["Cargo.toml", "libs/rust", "apps/account-service", "apps/cloud-service"],
		required_commands: ["pnpm check:api"],
	},
	{
		runtime: "TypeScript",
		scope: "frontends, SDKs, web runtime, and tooling",
		required_paths: ["pnpm-workspace.yaml", "libs/ts", "apps/account-web", "apps/cloud-web", "apps/console-web"],
		required_commands: ["pnpm check:web"],
	},
	{
		runtime: "Go",
		scope: "cloud control plane, operators, provisioning, and network services",
		required_paths: ["libs/go", "go.mod", "tools/go-workspace/project.json"],
		required_commands: ["pnpm check:go"],
	},
	{
		runtime: "Python",
		scope: "analytics pipelines, AI services, OCR, and data quality",
		required_paths: ["libs/python", "pyproject.toml", "tools/python-workspace/project.json"],
		required_commands: ["pnpm check:python"],
	},
];

function readJson(path) {
	if (!existsSync(path)) return undefined;
	try {
		return JSON.parse(readFileSync(path, "utf8"));
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return undefined;
	}
}

function commandConfigured(command, scripts) {
	if (command.startsWith("pnpm ")) return Boolean(scripts[command.slice("pnpm ".length)]);
	return false;
}

function statusFor(missingPaths, missingCommands) {
	return missingPaths.length === 0 && missingCommands.length === 0 ? "passed" : "failed";
}

function decisionFor(status) {
	return status === "passed" ? "go" : "no-go";
}

function evidenceFor(runtime, configuredCommands, missingPaths, missingCommands) {
	if (missingPaths.length > 0 || missingCommands.length > 0) return [];
	return [
		...runtime.required_paths.map((path) => `path:${path}`),
		...configuredCommands.map((command) => `command:${command}`),
	];
}

function proofFor(configuredCommands, missingPaths, missingCommands) {
	if (missingPaths.length > 0) return `missing required paths: ${missingPaths.join(", ")}`;
	if (missingCommands.length > 0) return `missing required commands: ${missingCommands.join(", ")}`;
	return configuredCommands.join(" && ");
}

function generatedEntries(scripts) {
	return required.map((runtime) => {
		const missing_paths = runtime.required_paths.filter((path) => !existsSync(path));
		const missing_commands = runtime.required_commands.filter((command) => !commandConfigured(command, scripts));
		const configured_commands = runtime.required_commands.filter((command) => commandConfigured(command, scripts));
		const status = statusFor(missing_paths, missing_commands);
		return {
			...runtime,
			owner: "Platform lead",
			status,
			decision: decisionFor(status),
			present_paths: runtime.required_paths.filter((path) => existsSync(path)),
			configured_commands,
			missing_paths,
			missing_commands,
			evidence: evidenceFor(runtime, configured_commands, missing_paths, missing_commands),
			proof: proofFor(configured_commands, missing_paths, missing_commands),
		};
	});
}

function keyFor(entry) {
	return entry.runtime;
}

function sameItems(actual, expected) {
	return Array.isArray(actual) && actual.length === expected.length && actual.every((item, index) => item === expected[index]);
}

function mergeExisting(generated, existing) {
	if (!existing?.runtimes) return generated;
	const byKey = new Map(existing.runtimes.map((entry) => [keyFor(entry), entry]));
	return generated.map((entry) => {
		const current = byKey.get(keyFor(entry));
		if (!current) return entry;
		return {
			...entry,
			owner: current.owner && !current.owner.endsWith(" required") ? current.owner : entry.owner,
			status: current.status && current.status !== "pending" ? current.status : entry.status,
			decision: current.decision === "go" ? current.decision : entry.decision,
			evidence: entry.evidence,
			proof: current.status === "accepted" ? current.proof : entry.proof,
		};
	});
}

function serializeJson(data) {
	return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
	const lines = [
		"# Runtime Foundation Ledger",
		"",
		"## Status",
		"",
		`- runtimes: ${data.summary.runtimes}`,
		`- pending: ${data.summary.pending}`,
		`- accepted: ${data.summary.accepted}`,
		`- passed: ${data.summary.passed}`,
		`- failed: ${data.summary.failed}`,
		`- missing_paths: ${data.summary.missing_paths}`,
		`- missing_commands: ${data.summary.missing_commands}`,
		"",
		"## Rules",
		"",
		"- Every runtime row must match the static runtime foundation contract.",
		"- `go` requires all required paths and commands to be present.",
		"- Missing paths, missing commands, evidence and proof must match generated repository state.",
		"- Summary counters must match runtime rows.",
		"- Strict cutover requires every runtime decision to be `go`.",
		"- Generation provenance must identify write and strict cutover commands.",
		"",
		"## Runtimes",
		"",
		"| Runtime | Status | Decision | Missing Paths | Missing Commands | Owner | Proof |",
		"|---|---:|---:|---:|---:|---|---|",
	];
	for (const runtime of data.runtimes) {
		lines.push(`| ${runtime.runtime} | ${runtime.status} | ${runtime.decision} | ${runtime.missing_paths.length} | ${runtime.missing_commands.length} | ${runtime.owner} | \`${runtime.proof}\` |`);
	}
	lines.push("", "## Regeneration", "", "```bash", "pnpm check:migration-runtime-foundation", "node tools/migration/runtime-foundation.mjs --write", "```", "");
	return lines.join("\n");
}

function validate(ledger, expected) {
	const expectedByKey = new Map(expected.map((entry) => [keyFor(entry), entry]));
	const expectedKeys = new Set(expectedByKey.keys());
	const seen = new Set();
	if (ledger.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (!Array.isArray(ledger.runtimes)) errors.push(`${outputPath}: runtimes must be an array`);
	for (const runtime of ledger.runtimes ?? []) {
		const key = keyFor(runtime);
		if (seen.has(key)) errors.push(`${outputPath}: duplicate runtime ${key}`);
		seen.add(key);
		const expectedRuntime = expectedByKey.get(key);
		if (!expectedRuntime) {
			errors.push(`${outputPath}: stale runtime ${key}`);
			continue;
		}
		if (runtime.scope !== expectedRuntime.scope) errors.push(`${key}: scope must match the runtime contract`);
		if (!sameItems(runtime.required_paths, expectedRuntime.required_paths)) {
			errors.push(`${key}: required_paths must match the runtime contract`);
		}
		if (!sameItems(runtime.required_commands, expectedRuntime.required_commands)) {
			errors.push(`${key}: required_commands must match the runtime contract`);
		}
		if (!sameItems(runtime.present_paths, expectedRuntime.present_paths)) {
			errors.push(`${key}: present_paths must match repository state`);
		}
		if (!sameItems(runtime.configured_commands, expectedRuntime.configured_commands)) {
			errors.push(`${key}: configured_commands must match package scripts`);
		}
		if (!sameItems(runtime.missing_paths, expectedRuntime.missing_paths)) {
			errors.push(`${key}: missing_paths must match repository state`);
		}
		if (!sameItems(runtime.missing_commands, expectedRuntime.missing_commands)) {
			errors.push(`${key}: missing_commands must match package scripts`);
		}
		if (!runtime.scope) errors.push(`${key}: scope is required`);
		if (!runtime.owner) errors.push(`${key}: owner is required`);
		if (!["pending", "passed", "accepted", "failed"].includes(runtime.status)) errors.push(`${key}: unsupported status ${runtime.status}`);
		if (!["go", "no-go"].includes(runtime.decision)) errors.push(`${key}: unsupported decision ${runtime.decision}`);
		if (!Array.isArray(runtime.evidence)) errors.push(`${key}: evidence must be an array`);
		if (!runtime.proof) errors.push(`${key}: proof is required`);
		if (runtime.status !== expectedRuntime.status && runtime.status !== "accepted") {
			errors.push(`${key}: status must be ${expectedRuntime.status} unless explicitly accepted`);
		}
		if (runtime.status === "accepted" && expectedRuntime.status !== "passed") {
			errors.push(`${key}: accepted status requires all runtime checks to pass first`);
		}
		if (runtime.decision !== expectedRuntime.decision) errors.push(`${key}: decision must be ${expectedRuntime.decision}`);
		if (!sameItems(runtime.evidence, expectedRuntime.evidence)) errors.push(`${key}: evidence must match repository state`);
		if (runtime.status !== "accepted" && runtime.proof !== expectedRuntime.proof) {
			errors.push(`${key}: proof must match repository state`);
		}
		if (strict) {
			if (runtime.owner.endsWith(" required")) errors.push(`${key}: owner must be assigned`);
			if (runtime.status !== "passed" && runtime.status !== "accepted") errors.push(`${key}: runtime must be passed or accepted`);
			if (runtime.decision !== "go") errors.push(`${key}: decision must be go`);
			if (runtime.missing_paths.length > 0) errors.push(`${key}: required paths missing: ${runtime.missing_paths.join(", ")}`);
			if (runtime.missing_commands.length > 0) errors.push(`${key}: required commands missing: ${runtime.missing_commands.join(", ")}`);
			if (runtime.evidence.length === 0) errors.push(`${key}: evidence must be attached`);
		}
	}
	for (const key of expectedKeys) {
		if (!seen.has(key)) errors.push(`${outputPath}: missing runtime ${key}`);
	}
	const summary = ledger.summary ?? {};
	const counts = { runtimes: ledger.runtimes?.length ?? 0, pending: 0, accepted: 0, passed: 0, failed: 0, missing_paths: 0, missing_commands: 0 };
	for (const runtime of ledger.runtimes ?? []) {
		if (Object.hasOwn(counts, runtime.status)) counts[runtime.status] += 1;
		counts.missing_paths += runtime.missing_paths?.length ?? 0;
		counts.missing_commands += runtime.missing_commands?.length ?? 0;
	}
	for (const [field, value] of Object.entries(counts)) {
		if (summary[field] !== value) errors.push(`${outputPath}: summary.${field} must be ${value}`);
	}
	if (ledger.generation?.command !== "node tools/migration/runtime-foundation.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (ledger.generation?.strict_cutover_command !== "node tools/migration/runtime-foundation.mjs --strict") {
		errors.push(`${outputPath}: generation.strict_cutover_command is invalid`);
	}
}

const generated = generatedEntries(readPackageScripts(errors));
const existing = readJson(outputPath);
const ledger = {
	schema_version: 1,
	generation: {
		command: "node tools/migration/runtime-foundation.mjs --write",
		source: "docs/blueprint/nvbes-full-restructure-big-bang-zero-debt.plan.md",
		strict_cutover_command: "node tools/migration/runtime-foundation.mjs --strict",
	},
	summary: { runtimes: generated.length, pending: 0, accepted: 0, passed: 0, failed: 0, missing_paths: 0, missing_commands: 0 },
	runtimes: mergeExisting(generated, existing),
};

for (const runtime of ledger.runtimes) {
	ledger.summary[runtime.status] += 1;
	ledger.summary.missing_paths += runtime.missing_paths.length;
	ledger.summary.missing_commands += runtime.missing_commands.length;
}

const json = serializeJson(ledger);
const markdown = serializeMarkdown(ledger);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration runtime foundation written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(ledger, generated);

for (const [path, expected] of [[outputPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/runtime-foundation.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/runtime-foundation.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration runtime foundation checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Migration runtime foundation: ok (${ledger.summary.runtimes} runtimes, ${ledger.summary.pending} pending)`);
