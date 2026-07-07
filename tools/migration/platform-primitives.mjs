#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const outputPath = "docs/migration/platform-primitives.generated.json";
const markdownPath = "docs/migration/platform-primitives.md";
const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const errors = [];
const packageScripts = readPackageScripts(errors);

const controls = [
	{
		id: "config",
		evidence: "libs/rust/core/src/config.rs",
		proof: "cargo test -p nvbes-core config --locked",
	},
	{
		id: "errors",
		evidence: "libs/rust/core/src/http.error.rs",
		proof: "cargo test -p nvbes-core app_error_constructors_keep_standard_error_contract --locked && cargo test -p nvbes-core rate_limit_error_preserves_retry_after --locked",
	},
	{
		id: "tenancy",
		evidence: "libs/rust/tenancy/src/lib.rs",
		proof: "cargo test -p nvbes-tenancy --locked",
	},
	{
		id: "audit",
		evidence: "libs/rust/audit/src/lib.rs",
		proof: "cargo test -p nvbes-audit --locked",
	},
	{
		id: "outbox",
		evidence: "libs/rust/platform/src/platform.outbox.rs",
		proof: "cargo test -p nvbes-platform --locked",
	},
	{
		id: "ports",
		evidence: "libs/rust/ports/src/lib.rs",
		proof: "cargo test -p nvbes-ports --locked",
	},
	{
		id: "observability",
		evidence: "libs/rust/observability/src/lib.rs",
		proof: "cargo test -p nvbes-observability --locked",
	},
	{
		id: "idempotency",
		evidence: "libs/rust/core/src/idempotency.rs",
		proof: "cargo test -p nvbes-core validate_key_trims_and_rejects_invalid_values --locked && cargo test -p nvbes-core request_signature_includes_method_path_and_body_hash --locked && cargo test -p nvbes-core derive_scope_changes_by_actor_and_route --locked",
	},
	{
		id: "events",
		evidence: "contracts/events/manifest.json",
		proof: "pnpm check:contracts",
	},
];

function buildLedger() {
	const entries = controls.map((control) => ({
		...control,
		status: existsSync(control.evidence) ? "active" : "missing",
	}));
	return {
		schema_version: 1,
		generation: {
			command: "node tools/migration/platform-primitives.mjs --write",
			strict_cutover_command: "node tools/migration/platform-primitives.mjs --strict",
		},
		summary: {
			entries: entries.length,
			active: entries.filter((entry) => entry.status === "active").length,
			missing: entries.filter((entry) => entry.status === "missing").length,
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
		"# Platform Primitive Ledger",
		"",
		"## Status",
		"",
		`- entries: ${ledger.summary.entries}`,
		`- active: ${ledger.summary.active}`,
		`- missing: ${ledger.summary.missing}`,
		`- pending: ${ledger.summary.pending}`,
		"",
		"## Rules",
		"",
		"- Every primitive row must match the static platform contract.",
		"- `active` requires the evidence path to exist.",
		"- Summary counters must match generated rows.",
		"- Strict cutover requires every primitive control to be active.",
		"- Generation provenance must identify write and strict cutover commands.",
		"",
		"## Controls",
		"",
		"| Control | Status | Proof | Evidence |",
		"|---|---:|---|---|",
	];
	for (const entry of ledger.entries) {
		lines.push(`| ${entry.id} | ${entry.status} | \`${entry.proof}\` | \`${entry.evidence}\` |`);
	}
	lines.push(
		"",
		"## Regeneration",
		"",
		"```bash",
		"pnpm check:migration-platform-primitives",
		"node tools/migration/platform-primitives.mjs --write",
		"```",
		"",
	);
	return lines.join("\n");
}

function validate(ledger) {
	const byId = new Map(controls.map((control) => [control.id, control]));
	const seen = new Set();
	if (ledger.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	if (!Array.isArray(ledger.entries)) errors.push(`${outputPath}: entries must be an array`);
	if ((ledger.entries ?? []).length !== controls.length) {
		errors.push(`${outputPath}: entries must include every platform primitive control`);
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
		if (entry.evidence !== expected.evidence) errors.push(`${entry.id}: evidence must be ${expected.evidence}`);
		if (entry.proof !== expected.proof) errors.push(`${entry.id}: proof must be ${expected.proof}`);
		errors.push(...validateProofCommand({ id: entry.id, proof: entry.proof }, packageScripts));
		if (!["active", "missing"].includes(entry.status)) errors.push(`${entry.id}: unsupported status ${entry.status}`);
		const expectedStatus = existsSync(expected.evidence) ? "active" : "missing";
		if (entry.status !== expectedStatus) errors.push(`${entry.id}: status must be ${expectedStatus}`);
		if (entry.status === "active" && !existsSync(expected.evidence)) {
			errors.push(`${entry.id}: evidence path missing: ${expected.evidence}`);
		}
		if (strict && entry.status !== "active") errors.push(`${entry.id}: primitive control missing`);
	}
	for (const control of controls) {
		if (!seen.has(control.id)) errors.push(`${outputPath}: missing control ${control.id}`);
	}
	const summary = ledger.summary ?? {};
	const active = (ledger.entries ?? []).filter((entry) => entry.status === "active").length;
	const missing = (ledger.entries ?? []).filter((entry) => entry.status === "missing").length;
	const pending = (ledger.entries ?? []).filter((entry) => entry.status !== "active").length;
	if (summary.entries !== controls.length) errors.push(`${outputPath}: summary.entries must be ${controls.length}`);
	if (summary.active !== active) errors.push(`${outputPath}: summary.active must be ${active}`);
	if (summary.missing !== missing) errors.push(`${outputPath}: summary.missing must be ${missing}`);
	if (summary.pending !== pending) errors.push(`${outputPath}: summary.pending must be ${pending}`);
	if (strict && pending !== 0) errors.push(`${outputPath}: strict cutover requires zero pending controls`);
	if (ledger.generation?.command !== "node tools/migration/platform-primitives.mjs --write") {
		errors.push(`${outputPath}: generation.command is invalid`);
	}
	if (ledger.generation?.strict_cutover_command !== "node tools/migration/platform-primitives.mjs --strict") {
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
	console.log(`Migration platform primitives written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(ledger);

for (const [path, expected] of [[outputPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run node tools/migration/platform-primitives.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run node tools/migration/platform-primitives.mjs --write`);
}

if (errors.length > 0) {
	console.error("Migration platform primitive checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Migration platform primitives: ok (${ledger.summary.entries} controls, ${ledger.summary.pending} pending)`,
);
