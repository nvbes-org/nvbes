#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/cutover-journal.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Cutover Journal",
	"## Status",
	"## Rules",
	"## Run Metadata",
	"## Timeline",
	"## Incident Entries",
	"## Decision",
];

const requiredMetadata = [
	"environment",
	"cutover window",
	"target commit",
	"target images",
	"snapshot id",
	"migration lead",
	"data lead",
	"infra lead",
	"support lead",
	"decision",
];

const requiredTimelineColumns = ["Time", "Phase", "Owner", "Action", "Evidence", "Result", "Decision"];
const requiredIncidentColumns = ["Time", "Severity", "Owner", "Symptom", "Impact", "Action", "Status"];
const requiredRules = ["Migration lead", "timestamp", "rollback decision", "immutable"];
const metadataColumns = ["Field", "Value"];
const validDecisions = ["go", "no-go"];
const validIncidentStatuses = ["pending", "no-go", "resolved", "accepted", "failed", "blocking"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const field of requiredMetadata) {
	if (!content.includes(`| ${field} |`)) errors.push(`missing cutover metadata: ${field}`);
}

for (const column of requiredTimelineColumns) {
	if (!content.includes(column)) errors.push(`missing timeline column: ${column}`);
}

for (const column of requiredIncidentColumns) {
	if (!content.includes(column)) errors.push(`missing incident column: ${column}`);
}

for (const rule of requiredRules) {
	if (!content.includes(rule)) errors.push(`missing journal rule: ${rule}`);
}

validateTable("## Run Metadata", metadataColumns, requiredMetadata);
validateTable("## Timeline", requiredTimelineColumns);
validateTable("## Incident Entries", requiredIncidentColumns);
validateStatusSummary();

const metadata = Object.fromEntries(tableRowsAfter("## Run Metadata").map(([field, value]) => [field, value]));
if (!validDecisions.includes(metadata.decision)) errors.push("cutover metadata decision must be go or no-go");
if (metadata.decision === "go") {
	for (const field of requiredMetadata.filter((field) => field !== "decision")) {
		if (isEmptyMarker(metadata[field])) errors.push(`go cutover decision requires ${field}`);
	}
	validateJournalGoDependencies();
}
for (const [time, , owner, action, evidence, result, decision] of tableRowsAfter("## Timeline")) {
	if (!validDecisions.includes(decision)) errors.push(`timeline entry has unsupported decision ${decision}`);
	if (decision === "go") {
		for (const [field, value] of [["time", time], ["owner", owner], ["action", action], ["evidence", evidence], ["result", result]]) {
			if (isEmptyMarker(value)) errors.push(`go timeline entry requires ${field}`);
		}
		if (!isParseableTimestamp(time)) errors.push(`go timeline entry requires parseable time: ${time}`);
		if (!hasConcreteEvidence(evidence)) errors.push(`go timeline entry requires concrete evidence`);
		if (!hasConcreteEvidence(result)) errors.push(`go timeline entry requires concrete result`);
	}
}
for (const [time, severity, owner, symptom, impact, action, status] of tableRowsAfter("## Incident Entries")) {
	if (!validIncidentStatuses.includes(status)) errors.push(`incident entry has unsupported status ${status}`);
	if (["resolved", "accepted"].includes(status)) {
		for (const [field, value] of [["time", time], ["severity", severity], ["owner", owner], ["symptom", symptom], ["impact", impact], ["action", action]]) {
			if (isEmptyMarker(value)) errors.push(`${status} incident entry requires ${field}`);
		}
		if (!isParseableTimestamp(time)) errors.push(`${status} incident entry requires parseable time: ${time}`);
		if (!hasConcreteEvidence(action)) errors.push(`${status} incident entry requires concrete action evidence`);
	}
}

if (!content.includes("Current decision: no-go.")) errors.push("missing current no-go journal decision");
if (!content.includes("This journal cannot approve cutover until all G0-G8 gates are `go`")) {
	errors.push("missing explicit journal cutover blocker");
}

if (strict) {
	const blockers = strictBlockers();
	for (const [blocker, count] of Object.entries(blockers)) {
		if (count > 0) errors.push(`${count} ${blocker} journal marker(s) remain`);
	}
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Field |") && !line.includes("Time |"))
		.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()));
}

function tableHeaderAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	const header = section.split("\n").find((line) => line.startsWith("|") && !line.includes("---"));
	return header ? header.split("|").slice(1, -1).map((cell) => cell.trim()) : [];
}

function validateTable(heading, expectedColumns, expectedLabels = undefined) {
	const header = tableHeaderAfter(heading);
	if (header.join("|") !== expectedColumns.join("|")) {
		errors.push(`${heading}: columns must be ${expectedColumns.join(", ")}`);
	}
	const rows = tableRowsAfter(heading);
	if (rows.length === 0) errors.push(`${heading}: must include at least one row`);
	for (const row of rows) {
		if (row.length !== expectedColumns.length) {
			errors.push(`${heading}: row ${row[0] ?? "unknown"} must have ${expectedColumns.length} columns`);
		}
	}
	if (!expectedLabels) return;
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
}

function validateStatusSummary() {
	const metadataRows = tableRowsAfter("## Run Metadata");
	const timelineRows = tableRowsAfter("## Timeline");
	const incidentRows = tableRowsAfter("## Incident Entries");
	const metadataValues = countValues(metadataRows.map(([, value]) => value), ["pending", "not run", "not scheduled"]);
	const metadataDecisions = countValues(metadataRows.filter(([field]) => field === "decision").map(([, value]) => value), validDecisions);
	const timelineDecisions = countValues(timelineRows.map((row) => row.at(-1)), validDecisions);
	const incidentStatuses = countValues(incidentRows.map((row) => row.at(-1)), validIncidentStatuses);
	const expected = {
		total_metadata_rows: metadataRows.length,
		pending_metadata_rows: metadataValues.pending,
		not_run_metadata_rows: metadataValues["not run"],
		not_scheduled_metadata_rows: metadataValues["not scheduled"],
		metadata_go_decisions: metadataDecisions.go,
		metadata_no_go_decisions: metadataDecisions["no-go"],
		total_timeline_rows: timelineRows.length,
		timeline_go_decisions: timelineDecisions.go,
		timeline_no_go_decisions: timelineDecisions["no-go"],
		total_incident_rows: incidentRows.length,
		incident_no_go_rows: incidentStatuses["no-go"],
		incident_resolved_rows: incidentStatuses.resolved,
		incident_accepted_rows: incidentStatuses.accepted,
		incident_failed_rows: incidentStatuses.failed,
		incident_blocking_rows: incidentStatuses.blocking,
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

function isEmptyMarker(value) {
	return ["", "pending", "none", "not run", "not scheduled", "no production action"].includes(value ?? "");
}

function isParseableTimestamp(value) {
	return !Number.isNaN(Date.parse(value ?? ""));
}

function hasConcreteEvidence(value) {
	return /(artifact|report|result|log|journal|record|incident|snapshot|commit|sha|digest|command|reconciliation|smoke|metric|health|link|url|id)/i.test(value ?? "");
}

function validateJournalGoDependencies() {
	const timelineRows = tableRowsAfter("## Timeline");
	const incidentRows = tableRowsAfter("## Incident Entries");
	for (const row of timelineRows) {
		if (row.at(-1) !== "go") errors.push(`go cutover decision requires ${row[0]} timeline entry`);
	}
	for (const row of incidentRows) {
		if (!["resolved", "accepted"].includes(row.at(-1))) {
			errors.push(`go cutover decision requires ${row[0]} incident entry resolved or accepted`);
		}
	}
}

function strictBlockers() {
	const counts = { pending: 0, "no-go": 0, "not run": 0, "not scheduled": 0 };
	for (const [, value] of tableRowsAfter("## Run Metadata")) {
		if (value === "pending") counts.pending += 1;
		if (value === "no-go") counts["no-go"] += 1;
		if (value === "not run") counts["not run"] += 1;
		if (value === "not scheduled") counts["not scheduled"] += 1;
	}
	for (const row of tableRowsAfter("## Timeline")) {
		if (row[0] === "not run") counts["not run"] += 1;
		if (row.at(-1) === "no-go") counts["no-go"] += 1;
	}
	for (const row of tableRowsAfter("## Incident Entries")) {
		if (row[0] === "not run") counts["not run"] += 1;
		if (row.at(-1) === "no-go") counts["no-go"] += 1;
	}
	return counts;
}

if (errors.length > 0) {
	console.error("Cutover journal checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`Cutover journal: ok (${requiredMetadata.length} metadata fields)`);
