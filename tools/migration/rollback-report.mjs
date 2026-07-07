#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/rollback-report.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Rollback Report",
	"## Status",
	"## Rules",
	"## Run Metadata",
	"## Steps",
	"## Decision",
];

const requiredMetadata = [
	"environment",
	"source_snapshot",
	"target_commit",
	"started_at",
	"completed_at",
	"duration",
	"max_duration",
	"decision",
];

const requiredSteps = [
	"stop new public stack",
	"stop new workers",
	"restore DNS/edge to legacy",
	"restore legacy read-only or read-write",
	"preserve cutover logs",
	"open incident record",
];

const metadataColumns = ["Field", "Value"];
const stepColumns = ["Step", "Evidence", "Status"];
const validStepStatuses = ["pending", "passed", "accepted", "failed", "blocking"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const field of requiredMetadata) {
	if (!content.includes(`| ${field} |`)) errors.push(`missing rollback metadata: ${field}`);
}

for (const step of requiredSteps) {
	if (!content.includes(`| ${step} |`)) errors.push(`missing rollback step: ${step}`);
}

validateTable("## Run Metadata", metadataColumns, requiredMetadata);
validateTable("## Steps", stepColumns, requiredSteps);
validateStatusSummary();

const metadata = Object.fromEntries(tableRowsAfter("## Run Metadata").map(([field, value]) => [field, value]));
const steps = tableRowsAfter("## Steps");
if (!metadata.decision || !["go", "no-go until rehearsed"].includes(metadata.decision)) {
	errors.push("rollback metadata decision must be go or no-go until rehearsed");
}
if (metadata.decision === "go") {
	for (const field of requiredMetadata.filter((field) => field !== "decision")) {
		if (["none", "pending", ""].includes(metadata[field])) errors.push(`go rollback decision requires ${field}`);
	}
	validateGoTimeline(metadata);
	validateGoDuration(metadata);
	for (const [step, , status] of steps) {
		if (!["passed", "accepted"].includes(status)) errors.push(`go rollback decision requires passed step: ${step}`);
	}
}
for (const [step, evidence, status] of steps) {
	if (!validStepStatuses.includes(status)) errors.push(`${step}: unsupported rollback status ${status}`);
	if (["passed", "accepted"].includes(status) && ["pending", "none", ""].includes(evidence)) {
		errors.push(`${step}: passed rollback step requires evidence`);
	}
	if (["passed", "accepted"].includes(status) && !hasConcreteEvidence(evidence)) {
		errors.push(`${step}: passed rollback step requires concrete evidence`);
	}
	if (["pending", "failed", "blocking"].includes(status) && metadata.decision === "go") {
		errors.push(`${step}: ${status} rollback step blocks go decision`);
	}
}

function validateStatusSummary() {
	const metadataRows = tableRowsAfter("## Run Metadata");
	const stepRows = tableRowsAfter("## Steps");
	const stepStatuses = countValues(stepRows.map((row) => row.at(-1)), validStepStatuses);
	const expected = {
		total_metadata_rows: metadataRows.length,
		none_metadata_rows: metadataRows.filter(([, value]) => value === "none").length,
		no_go_metadata_decisions: metadataRows.filter(([field, value]) => field === "decision" && value.startsWith("no-go")).length,
		total_step_rows: stepRows.length,
		pending_step_evidence_rows: stepRows.filter(([, evidence]) => evidence === "pending").length,
		pending_step_rows: stepStatuses.pending,
		passed_step_rows: stepStatuses.passed,
		accepted_step_rows: stepStatuses.accepted,
		failed_step_rows: stepStatuses.failed,
		blocking_step_rows: stepStatuses.blocking,
	};
	for (const [key, value] of Object.entries(expected)) {
		const actual = statusNumber(key);
		if (actual === null) errors.push(`missing status summary: ${key}`);
		else if (actual !== value) errors.push(`${key}: expected ${value}, found ${actual}`);
	}
}

function validateGoDuration(metadata) {
	const duration = durationMinutes(metadata.duration);
	const maxDuration = durationMinutes(metadata.max_duration);
	if (duration === null) errors.push("go rollback decision requires parseable duration");
	if (maxDuration === null) errors.push("go rollback decision requires parseable max_duration");
	if (duration !== null && maxDuration !== null && duration > maxDuration) {
		errors.push(`go rollback duration ${metadata.duration} exceeds max_duration ${metadata.max_duration}`);
	}
}

function validateGoTimeline(metadata) {
	const startedAt = Date.parse(metadata.started_at ?? "");
	const completedAt = Date.parse(metadata.completed_at ?? "");
	if (Number.isNaN(startedAt)) errors.push("go rollback decision requires parseable started_at");
	if (Number.isNaN(completedAt)) errors.push("go rollback decision requires parseable completed_at");
	if (!Number.isNaN(startedAt) && !Number.isNaN(completedAt) && completedAt < startedAt) {
		errors.push("go rollback decision requires completed_at at or after started_at");
	}
}

function durationMinutes(value) {
	const text = String(value ?? "").trim().toLowerCase();
	const minuteMatch = text.match(/^(\d+)\s*(m|min|minutes)$/);
	if (minuteMatch) return Number(minuteMatch[1]);
	const isoMatch = text.match(/^pt(?:(\d+)h)?(?:(\d+)m)?$/);
	if (isoMatch) return Number(isoMatch[1] ?? 0) * 60 + Number(isoMatch[2] ?? 0);
	const clockMatch = text.match(/^(\d{1,2}):(\d{2})(?::(\d{2}))?$/);
	if (clockMatch) return Number(clockMatch[1]) * 60 + Number(clockMatch[2]) + (Number(clockMatch[3] ?? 0) > 0 ? 1 : 0);
	return null;
}

function hasConcreteEvidence(value) {
	return /(artifact|report|result|log|journal|record|incident|snapshot|commit|sha|dns|edge|worker|health|command output|link|url|id)/i.test(value ?? "");
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

if (!content.includes("Rollback is not approved until a rehearsal report replaces this template.")) {
	errors.push("missing rollback template blocker");
}

if (strict) {
	const blockers = strictBlockers();
	for (const [blocker, count] of Object.entries(blockers)) {
		if (count > 0) errors.push(`${count} ${blocker} rollback marker(s) remain`);
	}
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Field") && !line.includes("Step |"))
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

function strictBlockers() {
	const counts = { pending: 0, blocking: 0, "no-go": 0, none: 0 };
	for (const [field, value] of tableRowsAfter("## Run Metadata")) {
		if (value === "pending") counts.pending += 1;
		if (value === "none") counts.none += 1;
		if (field === "decision" && value.startsWith("no-go")) counts["no-go"] += 1;
	}
	for (const [, evidence, status] of tableRowsAfter("## Steps")) {
		if (evidence === "pending") counts.pending += 1;
		if (evidence === "none") counts.none += 1;
		if (status === "pending") counts.pending += 1;
		if (status === "blocking") counts.blocking += 1;
	}
	return counts;
}

if (errors.length > 0) {
	console.error("Rollback report checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Rollback report: ok (${requiredSteps.length} rollback steps)`);
