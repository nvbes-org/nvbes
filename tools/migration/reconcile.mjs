#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { optionValue } from "./cli-options.mjs";

const args = process.argv.slice(2);

if (args.includes("--help")) {
	console.log("Usage: node tools/migration/reconcile.mjs --env <env> --report <path> [--allow-template]");
	process.exit(0);
}

const env = optionValue(args, "--env");
const report = optionValue(args, "--report");
const allowTemplate = args.includes("--allow-template");
const errors = [];

if (!env) errors.push("--env is required");
if (!report) errors.push("--report is required");
if (report && !existsSync(report)) errors.push(`${report}: report file not found`);

function validateMarkdownReport(content) {
	const requiredSections = ["# Reconciliation Report", "## Run Metadata", "## Summary", "## Decision"];
	for (const section of requiredSections) {
		if (!content.includes(section)) errors.push(`${report}: missing section "${section}"`);
	}

	if (!allowTemplate && /Status\s*\n\s*Template initialized/i.test(content)) {
		errors.push(`${report}: template report cannot approve ${env}`);
	}

	if (!allowTemplate && /\|\s*blocking\s*\|/i.test(content)) {
		errors.push(`${report}: blocking reconciliation status present`);
	}

	if (!allowTemplate && /decision\s*\|\s*no-go/i.test(content)) {
		errors.push(`${report}: no-go decision present`);
	}
}

function requireObject(value, path) {
	if (!value || typeof value !== "object" || Array.isArray(value)) {
		errors.push(`${path}: object expected`);
		return false;
	}
	return true;
}

function requireNumber(value, path) {
	if (typeof value !== "number" || !Number.isFinite(value) || value < 0) {
		errors.push(`${path}: non-negative number expected`);
	}
}

function requireInteger(value, path) {
	if (!Number.isInteger(value) || value < 0) {
		errors.push(`${path}: non-negative integer expected`);
	}
}

function validateJsonReport(content) {
	let data;
	try {
		data = JSON.parse(content);
	} catch (error) {
		errors.push(`${report}: invalid JSON: ${error.message}`);
		return;
	}

	if (!requireObject(data, "report")) return;
	if (data.schema_version !== 1) errors.push("schema_version: expected 1");
	if (data.environment !== env) errors.push(`environment: expected ${env}`);
	if (!data.snapshot_id) errors.push("snapshot_id: required");
	if (!requireObject(data.target, "target")) return;
	if (!data.target.commit) errors.push("target.commit: required");
	if (!data.target.contracts_version) errors.push("target.contracts_version: required");

	if (requireObject(data.durations_seconds, "durations_seconds")) {
		for (const key of ["export", "transform", "import", "reconciliation"]) {
			requireNumber(data.durations_seconds[key], `durations_seconds.${key}`);
		}
	}

	if (!Array.isArray(data.domains) || data.domains.length === 0) {
		errors.push("domains: non-empty array expected");
	} else {
		for (const [index, domain] of data.domains.entries()) {
			const path = `domains[${index}]`;
			if (!requireObject(domain, path)) continue;
			if (!domain.name) errors.push(`${path}.name: required`);
			if (requireObject(domain.counts, `${path}.counts`)) {
				requireInteger(domain.counts.source_rows, `${path}.counts.source_rows`);
				requireInteger(domain.counts.target_rows, `${path}.counts.target_rows`);
				if (!Number.isInteger(domain.counts.delta)) errors.push(`${path}.counts.delta: integer expected`);
				if (!allowTemplate && domain.counts.delta !== 0 && domain.status !== "accepted") {
					errors.push(`${path}: row count delta requires accepted status`);
				}
			}
			if (requireObject(domain.checksums, `${path}.checksums`)) {
				if (typeof domain.checksums.matched !== "boolean") {
					errors.push(`${path}.checksums.matched: boolean expected`);
				}
				if (!allowTemplate && domain.checksums.matched !== true && domain.status !== "accepted") {
					errors.push(`${path}: checksum mismatch requires accepted status`);
				}
			}
			requireInteger(domain.orphans, `${path}.orphans`);
			requireInteger(domain.duplicates, `${path}.duplicates`);
			if (!allowTemplate && domain.orphans > 0 && domain.status !== "accepted") {
				errors.push(`${path}: orphans require accepted status`);
			}
			if (!allowTemplate && domain.duplicates > 0 && domain.status !== "accepted") {
				errors.push(`${path}: duplicates require accepted status`);
			}
			if (requireObject(domain.rejects, `${path}.rejects`)) {
				for (const key of ["fixed", "accepted", "rejected", "blocking"]) {
					requireInteger(domain.rejects[key], `${path}.rejects.${key}`);
				}
				if (!allowTemplate && domain.rejects.blocking > 0) {
					errors.push(`${path}: blocking rejects present`);
				}
			}
			if (!["passed", "accepted", "blocking"].includes(domain.status)) {
				errors.push(`${path}.status: unsupported status`);
			}
			if (!allowTemplate && domain.status === "blocking") {
				errors.push(`${path}: blocking status present`);
			}
		}
	}

	if (!["go", "no-go", "go-with-accepted-rejects"].includes(data.decision)) {
		errors.push("decision: unsupported value");
	}
	if (!allowTemplate && data.decision === "no-go") {
		errors.push("decision: no-go present");
	}
}

function validateSchemaFile() {
	const schemaPath = "docs/migration/reconciliation-report.schema.json";
	if (!existsSync(schemaPath)) {
		errors.push(`${schemaPath}: missing`);
		return;
	}
	const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
	if (schema.properties?.schema_version?.const !== 1) {
		errors.push(`${schemaPath}: schema_version const must be 1`);
	}
	if (!schema.required?.includes("domains") || !schema.required?.includes("decision")) {
		errors.push(`${schemaPath}: domains and decision must be required`);
	}
}

if (errors.length === 0) {
	const content = readFileSync(report, "utf8");
	if (report.endsWith(".json")) {
		validateSchemaFile();
		validateJsonReport(content);
	} else {
		validateMarkdownReport(content);
	}
}

if (errors.length > 0) {
	console.error("Migration reconciliation failed:");
	for (const error of errors) {
		console.error(`- ${error}`);
	}
	process.exit(1);
}

console.log(`Migration reconciliation: ok (${env}, ${report})`);
