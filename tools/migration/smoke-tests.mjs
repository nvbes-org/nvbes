#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/smoke-test-manifest.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Smoke Test Manifest",
	"## Status",
	"## Rules",
	"## Critical Journeys",
	"## Runtime Evidence",
	"## Decision",
];

const requiredJourneys = [
	"login",
	"MFA",
	"workspace access",
	"upload/download",
	"share link",
	"billing entitlement",
	"webhook replay",
	"privacy export",
	"audit event",
	"worker queue",
];

const requiredEvidence = [
	"target version",
	"environment proof",
	"logs",
	"metrics",
	"rollback still possible",
];

const journeyColumns = ["Journey", "Owner", "Command", "Environment", "Evidence", "Status", "Decision"];
const evidenceColumns = ["Evidence", "Required content", "Status"];
const validStatuses = ["pending", "passed", "accepted", "failed", "blocking"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const journey of requiredJourneys) {
	if (!content.includes(`| ${journey} |`)) errors.push(`missing smoke journey: ${journey}`);
}

for (const evidence of requiredEvidence) {
	if (!content.includes(`| ${evidence} |`)) errors.push(`missing runtime evidence: ${evidence}`);
}

validateTable("## Critical Journeys", journeyColumns, requiredJourneys);
validateTable("## Runtime Evidence", evidenceColumns, requiredEvidence);
validateStatusSummary();

const runtimeEvidenceRows = tableRowsAfter("## Runtime Evidence");

for (const row of tableRowsAfter("## Critical Journeys")) {
	validateJourneyRow(row);
}
for (const [evidence, , status] of runtimeEvidenceRows) {
	if (!validStatuses.includes(status)) {
		errors.push(`${evidence}: unsupported runtime evidence status ${status}`);
	}
}
for (const row of runtimeEvidenceRows) {
	validateRuntimeEvidence(row);
}

if (!content.includes("Cutover remains no-go until every critical journey and runtime evidence row")) {
	errors.push("missing explicit smoke no-go decision");
}

if (strict) {
	const blockers = strictBlockers();
	for (const [blocker, count] of Object.entries(blockers)) {
		if (count > 0) errors.push(`${count} ${blocker} smoke-test marker(s) remain`);
	}
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Journey |") && !line.includes("Evidence |"))
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

function validateJourneyRow(row) {
	const [journey, owner, command, environment, evidence, status, decision] = row;
	if (["pending", "unassigned", "none", ""].includes(owner)) {
		errors.push(`${journey}: owner is required`);
	}
	if (!validStatuses.includes(status)) {
		errors.push(`${journey}: unsupported journey status ${status}`);
	}
	if (!["go", "no-go"].includes(decision)) errors.push(`${journey}: unsupported journey decision ${decision}`);
	if (decision === "go") {
		for (const [field, value] of [["command", command], ["environment", environment], ["evidence", evidence]]) {
			if (["pending", "none", ""].includes(value)) errors.push(`${journey}: go decision requires ${field}`);
		}
		if (!["passed", "accepted"].includes(status)) errors.push(`${journey}: go decision requires passed or accepted status`);
		for (const [runtimeEvidence, , runtimeStatus] of runtimeEvidenceRows) {
			if (!["passed", "accepted"].includes(runtimeStatus)) {
				errors.push(`${journey}: go decision requires ${runtimeEvidence} runtime evidence`);
			}
		}
	}
	if (["pending", "failed", "blocking"].includes(status) && decision !== "no-go") {
		errors.push(`${journey}: ${status} status requires no-go decision`);
	}
}

function validateStatusSummary() {
	const journeys = tableRowsAfter("## Critical Journeys");
	const runtimeEvidence = tableRowsAfter("## Runtime Evidence");
	const journeyStatuses = countValues(journeys.map((row) => row.at(-2)), validStatuses);
	const journeyDecisions = countValues(journeys.map((row) => row.at(-1)), ["go", "no-go"]);
	const runtimeStatuses = countValues(runtimeEvidence.map((row) => row.at(-1)), validStatuses);
	const expected = {
		total_journey_rows: journeys.length,
		pending_journey_rows: journeyStatuses.pending,
		passed_journey_rows: journeyStatuses.passed,
		accepted_journey_rows: journeyStatuses.accepted,
		failed_journey_rows: journeyStatuses.failed,
		blocking_journey_rows: journeyStatuses.blocking,
		go_decisions: journeyDecisions.go,
		no_go_decisions: journeyDecisions["no-go"],
		total_runtime_evidence_rows: runtimeEvidence.length,
		pending_runtime_evidence_rows: runtimeStatuses.pending,
		passed_runtime_evidence_rows: runtimeStatuses.passed,
		accepted_runtime_evidence_rows: runtimeStatuses.accepted,
		failed_runtime_evidence_rows: runtimeStatuses.failed,
		blocking_runtime_evidence_rows: runtimeStatuses.blocking,
	};
	for (const [key, value] of Object.entries(expected)) {
		const actual = statusNumber(key);
		if (actual === null) errors.push(`missing status summary: ${key}`);
		else if (actual !== value) errors.push(`${key}: expected ${value}, found ${actual}`);
	}
}

function validateRuntimeEvidence(row) {
	const [evidence, requiredContent, status] = row;
	if (!["passed", "accepted"].includes(status)) return;
	if (!hasConcreteEvidence(requiredContent)) {
		errors.push(`${evidence}: passed/accepted runtime evidence requires concrete artifact content`);
	}
}

function hasConcreteEvidence(value) {
	const normalized = (value ?? "").toLowerCase();
	return /(artifact|commit|digest|migration|contract|url|version|region|log|request id|correlation id|metric|latency|lag|health|window|report|result)/.test(normalized);
}

function strictBlockers() {
	const counts = { pending: 0, unassigned: 0, "no-go": 0 };
	for (const [, owner, , , , status, decision] of tableRowsAfter("## Critical Journeys")) {
		if (status === "pending") counts.pending += 1;
		if (owner === "unassigned") counts.unassigned += 1;
		if (decision === "no-go") counts["no-go"] += 1;
	}
	for (const [, , status] of tableRowsAfter("## Runtime Evidence")) {
		if (status === "pending") counts.pending += 1;
	}
	return counts;
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

if (errors.length > 0) {
	console.error("Smoke test checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Smoke tests: ok (${requiredJourneys.length} journeys, ${requiredEvidence.length} evidence rows)`,
);
