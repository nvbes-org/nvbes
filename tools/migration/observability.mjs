#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/observability-readiness.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Observability Readiness",
	"## Status",
	"## Rules",
	"## Critical Signals",
	"## Cutover Evidence",
	"## Decision",
];

const requiredSignals = [
	"API health",
	"auth health",
	"database health",
	"queue health",
	"worker health",
	"edge health",
	"billing health",
	"audit health",
];

const requiredEvidence = [
	"dashboard set",
	"alert test",
	"log trace sample",
	"SLO snapshot",
	"incident channel",
];

const signalColumns = ["Signal", "Scope", "Owner", "Evidence", "Alert path", "Status", "Decision"];
const evidenceColumns = ["Evidence", "Required content", "Status", "Decision"];
const validStatuses = ["pending", "passed", "accepted", "failed", "blocking"];
const validDecisions = ["go", "no-go"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const signal of requiredSignals) {
	if (!content.includes(`| ${signal} |`)) errors.push(`missing critical signal: ${signal}`);
}

for (const evidence of requiredEvidence) {
	if (!content.includes(`| ${evidence} |`)) {
		errors.push(`missing cutover evidence: ${evidence}`);
	}
}

validateTable("## Critical Signals", signalColumns, requiredSignals);
validateTable("## Cutover Evidence", evidenceColumns, requiredEvidence);
validateStatusSummary();

const cutoverEvidenceRows = tableRowsAfter("## Cutover Evidence");

for (const row of tableRowsAfter("## Critical Signals")) {
	validateSignalRow(row);
}
for (const row of cutoverEvidenceRows) {
	validateDecisionRow(row[0], row.at(-2), row.at(-1));
	validateCutoverEvidence(row);
}

if (strict) {
	const blockers = strictBlockers();
	for (const [blocker, count] of Object.entries(blockers)) {
		if (count > 0) errors.push(`${count} ${blocker} observability marker(s) remain`);
	}
}

if (!content.includes("Cutover remains no-go until every critical signal and cutover evidence row is")) {
	errors.push("missing explicit observability no-go decision");
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Signal |") && !line.includes("Evidence |"))
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

function validateSignalRow(row) {
	const [signal, , owner, evidence, alertPath, status, decision] = row;
	if (decision === "go") {
		for (const [field, value] of [["owner", owner], ["evidence", evidence], ["alert path", alertPath]]) {
			if (["pending", "none", ""].includes(value)) errors.push(`${signal}: go decision requires ${field}`);
		}
		for (const [cutoverEvidence, , status] of cutoverEvidenceRows) {
			if (!["passed", "accepted"].includes(status)) {
				errors.push(`${signal}: go decision requires ${cutoverEvidence} cutover evidence`);
			}
		}
	}
	validateDecisionRow(signal, status, decision);
}

function validateStatusSummary() {
	const rows = [...tableRowsAfter("## Critical Signals"), ...tableRowsAfter("## Cutover Evidence")];
	const statusCounts = Object.fromEntries(validStatuses.map((status) => [status, 0]));
	const decisionCounts = Object.fromEntries(validDecisions.map((decision) => [decision, 0]));
	for (const row of rows) {
		statusCounts[row.at(-2)] = (statusCounts[row.at(-2)] ?? 0) + 1;
		decisionCounts[row.at(-1)] = (decisionCounts[row.at(-1)] ?? 0) + 1;
	}
	const expected = {
		total_decision_rows: rows.length,
		pending_rows: statusCounts.pending,
		passed_rows: statusCounts.passed,
		accepted_rows: statusCounts.accepted,
		failed_rows: statusCounts.failed,
		blocking_rows: statusCounts.blocking,
		go_decisions: decisionCounts.go,
		no_go_decisions: decisionCounts["no-go"],
	};
	for (const [key, value] of Object.entries(expected)) {
		const actual = statusNumber(key);
		if (actual === null) errors.push(`missing status summary: ${key}`);
		else if (actual !== value) errors.push(`${key}: expected ${value}, found ${actual}`);
	}
}

function statusNumber(key) {
	const match = content.match(new RegExp(`- \`${key}\`: (\\d+)`));
	return match ? Number.parseInt(match[1], 10) : null;
}

function validateDecisionRow(label, status, decision) {
	if (!validStatuses.includes(status)) {
		errors.push(`${label}: unsupported observability status ${status}`);
	}
	if (!validDecisions.includes(decision)) errors.push(`${label}: unsupported observability decision ${decision}`);
	if (decision === "go" && !["passed", "accepted"].includes(status)) {
		errors.push(`${label}: go decision requires passed or accepted status`);
	}
	if (["pending", "failed", "blocking"].includes(status) && decision !== "no-go") {
		errors.push(`${label}: ${status} status requires no-go decision`);
	}
}

function validateCutoverEvidence(row) {
	const [evidence, requiredContent, status, decision] = row;
	if (decision !== "go" && !["passed", "accepted"].includes(status)) return;
	if (!hasConcreteEvidence(requiredContent)) {
		errors.push(`${evidence}: go/passed cutover evidence requires concrete artifact content`);
	}
}

function hasConcreteEvidence(value) {
	const normalized = (value ?? "").toLowerCase();
	return /(artifact|dashboard|alert|incident|log|trace|sample|snapshot|report|result|link|url|metric|slo|acknowledg)/.test(normalized);
}

function strictBlockers() {
	const counts = { pending: 0, blocking: 0, failed: 0, "no-go": 0 };
	for (const row of [...tableRowsAfter("## Critical Signals"), ...tableRowsAfter("## Cutover Evidence")]) {
		const status = row.at(-2);
		const decision = row.at(-1);
		if (status === "pending") counts.pending += 1;
		if (status === "blocking") counts.blocking += 1;
		if (status === "failed") counts.failed += 1;
		if (decision === "no-go") counts["no-go"] += 1;
	}
	return counts;
}

if (errors.length > 0) {
	console.error("Observability checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Observability: ok (${requiredSignals.length} signals, ${requiredEvidence.length} evidence rows)`,
);
