#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/reconciliation-report.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Reconciliation Report",
	"## Status",
	"## Rules",
	"## Run Metadata",
	"## Summary",
	"## Evidence",
	"## Decision",
];

const requiredMetadata = [
	"environment",
	"snapshot_id",
	"target_commit",
	"started_at",
	"completed_at",
	"decision",
];

const requiredDomains = [
	"Identity",
	"Workspace/Authz",
	"Drive",
	"Billing/Usage",
	"Audit/Privacy",
	"Developer Platform",
];

const requiredSummaryColumns = ["Domain", "Counts", "Checksums", "Orphans", "Rejects", "Status"];
const metadataColumns = ["Field", "Value"];
const requiredRules = [
	"Markdown `go` decisions",
	"summary domains",
	"blocking domain",
	"accepted domain",
];
const requiredCommands = [
	"tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json",
	"tools/migration/reconcile.mjs --env template --report docs/migration/reconciliation.template.json --allow-template",
];

const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const field of requiredMetadata) {
	if (!content.includes(`| ${field} |`)) errors.push(`missing reconciliation metadata: ${field}`);
}

for (const domain of requiredDomains) {
	if (!content.includes(`| ${domain} |`)) errors.push(`missing reconciliation domain: ${domain}`);
}

for (const column of requiredSummaryColumns) {
	if (!content.includes(column)) errors.push(`missing summary column: ${column}`);
}

for (const rule of requiredRules) {
	if (!content.includes(rule)) errors.push(`missing reconciliation rule: ${rule}`);
}

for (const command of requiredCommands) {
	if (!content.includes(command)) errors.push(`missing validation command: ${command}`);
}

validateTable("## Run Metadata", metadataColumns, requiredMetadata);
validateTable("## Summary", requiredSummaryColumns, requiredDomains);
validateStatusSummary();
validateMetadata();
const summaryRows = tableRowsAfter("## Summary");
for (const row of summaryRows) {
	validateSummaryRow(row);
}

if (strict) {
	const blockers = [...content.matchAll(/\b(pending|blocking|no-go|none)\b/gi)].map((match) =>
		match[1].toLowerCase(),
	);
	if (blockers.length > 0) {
		const counts = blockers.reduce((acc, blocker) => {
			acc[blocker] = (acc[blocker] ?? 0) + 1;
			return acc;
		}, {});
		for (const [blocker, count] of Object.entries(counts)) {
			errors.push(`${count} ${blocker} reconciliation marker(s) remain`);
		}
	}
}

if (!content.includes("No cutover decision is approved from this template.")) {
	errors.push("missing explicit reconciliation template blocker");
}

function validateMetadata() {
	const fields = Object.fromEntries(tableRowsAfter("## Run Metadata"));
	const decision = normalizeDecision(fields.decision);
	if (!["go", "go-with-accepted-rejects", "no-go"].includes(decision)) {
		errors.push(`decision: unsupported reconciliation decision ${fields.decision}`);
		return;
	}
	if (["go", "go-with-accepted-rejects"].includes(decision)) {
		for (const field of ["environment", "snapshot_id", "target_commit", "started_at", "completed_at"]) {
			if (isEmptyMarker(fields[field])) errors.push(`${field}: ${decision} requires concrete metadata`);
		}
		validateMetadataTimeline(fields, decision);
		for (const row of summaryRows) {
			if (!["passed", "accepted"].includes(row.at(-1))) {
				errors.push(`decision: ${decision} requires ${row[0]} reconciliation domain`);
			}
		}
	}
}

function validateMetadataTimeline(fields, decision) {
	const startedAt = Date.parse(fields.started_at ?? "");
	const completedAt = Date.parse(fields.completed_at ?? "");
	if (Number.isNaN(startedAt)) errors.push(`started_at: ${decision} requires a parseable timestamp`);
	if (Number.isNaN(completedAt)) errors.push(`completed_at: ${decision} requires a parseable timestamp`);
	if (!Number.isNaN(startedAt) && !Number.isNaN(completedAt) && completedAt < startedAt) {
		errors.push(`completed_at: ${decision} requires completion after start`);
	}
}

function validateSummaryRow(row) {
	const [domain, counts, checksums, orphans, rejects, status] = row;
	if (!["pending", "passed", "accepted", "blocking"].includes(status)) {
		errors.push(`${domain}: unsupported reconciliation status ${status}`);
	}
	if (["passed", "accepted"].includes(status)) {
		for (const [field, value] of [
			["counts", counts],
			["checksums", checksums],
			["orphans", orphans],
			["rejects", rejects],
		]) {
			if (isEmptyMarker(value)) errors.push(`${domain}: ${status} reconciliation requires ${field}`);
		}
	}
}

function validateStatusSummary() {
	const metadataRows = tableRowsAfter("## Run Metadata");
	const summaryRows = tableRowsAfter("## Summary");
	const metadataValues = countValues(metadataRows.map(([, value]) => normalizeDecision(value)), ["none", "no-go"]);
	const domainStatuses = countValues(summaryRows.map((row) => row.at(-1)), ["pending", "passed", "accepted", "blocking"]);
	const expected = {
		total_metadata_rows: metadataRows.length,
		none_metadata_rows: metadataValues.none,
		metadata_no_go_decisions: metadataValues["no-go"],
		total_domain_rows: summaryRows.length,
		pending_domain_rows: domainStatuses.pending,
		passed_domain_rows: domainStatuses.passed,
		accepted_domain_rows: domainStatuses.accepted,
		blocking_domain_rows: domainStatuses.blocking,
	};
	for (const [key, value] of Object.entries(expected)) {
		const actual = statusNumber(key);
		if (actual === null) errors.push(`missing status summary: ${key}`);
		else if (actual !== value) errors.push(`${key}: expected ${value}, found ${actual}`);
	}
}

function countValues(values, knownValues) {
	const counts = Object.fromEntries(knownValues.map((value) => [value, 0]));
	for (const value of values) counts[value] = (counts[value] ?? 0) + 1;
	return counts;
}

function statusNumber(key) {
	const match = content.match(new RegExp(`- \`${key}\`: (\\d+)`));
	return match ? Number.parseInt(match[1], 10) : null;
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---"))
		.filter((line) => !line.includes("Field |") && !line.includes("Domain |"))
		.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()));
}

function tableHeaderAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	const header = section.split("\n").find((line) => line.startsWith("|") && !line.includes("---"));
	return header ? header.split("|").slice(1, -1).map((cell) => cell.trim()) : [];
}

function validateTable(heading, expectedColumns, expectedLabels) {
	const header = tableHeaderAfter(heading);
	if (header.join("|") !== expectedColumns.join("|")) {
		errors.push(`${heading}: columns must be ${expectedColumns.join(", ")}`);
	}
	const rows = tableRowsAfter(heading);
	if (rows.length !== expectedLabels.length) {
		errors.push(`${heading}: expected ${expectedLabels.length} rows, found ${rows.length}`);
	}
	const labels = rows.map((row) => row[0]);
	for (const label of expectedLabels) {
		if (!labels.includes(label)) errors.push(`${heading}: missing row ${label}`);
	}
	for (const label of labels) {
		if (!expectedLabels.includes(label)) errors.push(`${heading}: unexpected row ${label}`);
	}
	for (const row of rows) {
		if (row.length !== expectedColumns.length) {
			errors.push(`${heading}: row ${row[0] ?? "unknown"} must have ${expectedColumns.length} columns`);
		}
	}
}

function normalizeDecision(value) {
	if (value === "go" || value === "go-with-accepted-rejects") return value;
	if (value?.startsWith("no-go")) return "no-go";
	return value ?? "";
}

function isEmptyMarker(value) {
	return ["", "none", "pending", "blocking", "no-go"].includes(value ?? "");
}

if (errors.length > 0) {
	console.error("Reconciliation report checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Reconciliation report: ok (${requiredDomains.length} domains)`);
