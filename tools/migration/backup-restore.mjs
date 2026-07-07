#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/backup-restore-manifest.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Backup Restore Manifest",
	"## Status",
	"## Rules",
	"## Backup Targets",
	"## Restore Evidence",
	"## Decision",
];

const requiredTargets = [
	"PostgreSQL identity",
	"PostgreSQL drive",
	"object storage",
	"Valkey",
	"event log",
	"audit store",
];

const requiredEvidence = [
	"restore environment",
	"restore duration",
	"integrity checks",
	"access controls",
	"retention proof",
];

const targetColumns = ["Target", "Scope", "Owner", "Backup proof", "Restore command", "RPO", "RTO", "Status", "Decision"];
const evidenceColumns = ["Evidence", "Required content", "Status", "Decision"];
const validStatuses = ["pending", "passed", "accepted", "failed", "blocking"];
const validDecisions = ["go", "no-go"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const target of requiredTargets) {
	if (!content.includes(`| ${target} |`)) errors.push(`missing backup target: ${target}`);
}

for (const evidence of requiredEvidence) {
	if (!content.includes(`| ${evidence} |`)) {
		errors.push(`missing restore evidence: ${evidence}`);
	}
}

validateTable("## Backup Targets", targetColumns, requiredTargets);
validateTable("## Restore Evidence", evidenceColumns, requiredEvidence);
validateStatusSummary();

const backupTargetRows = tableRowsAfter("## Backup Targets");
const restoreEvidenceRows = tableRowsAfter("## Restore Evidence");

for (const row of backupTargetRows) {
	validateBackupTarget(row);
}
for (const row of restoreEvidenceRows) {
	validateRestoreEvidenceRow(row);
}

if (strict) {
	const blockers = strictBlockers();
	for (const [blocker, count] of Object.entries(blockers)) {
		if (count > 0) errors.push(`${count} ${blocker} backup/restore marker(s) remain`);
	}
}

if (!content.includes("Production cutover remains no-go until every backup target and restore evidence")) {
	errors.push("missing explicit backup/restore blocker");
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Target |") && !line.includes("Evidence |"))
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

function validateBackupTarget(row) {
	const [target, scope, owner, backupProof, restoreCommand, rpo, rto, status, decision] = row;
	if (["pending", "none", ""].includes(scope)) errors.push(`${target}: scope is required`);
	if (["pending", "none", ""].includes(owner)) errors.push(`${target}: owner is required`);
	if (decision === "go") {
		for (const [field, value] of [["backup proof", backupProof], ["restore command", restoreCommand], ["RPO", rpo], ["RTO", rto]]) {
			if (["pending", "none", ""].includes(value)) errors.push(`${target}: go decision requires ${field}`);
		}
	}
	validateDecisionRow(target, status, decision);
}

function validateRestoreEvidenceRow(row) {
	const [evidence, requiredContent, status, decision] = row;
	validateDecisionRow(evidence, status, decision);
	if (decision !== "go") return;
	if (!hasConcreteEvidence(requiredContent)) {
		errors.push(`${evidence}: go restore evidence requires concrete artifact content`);
	}
	for (const [target, , , , , , , , targetDecision] of backupTargetRows) {
		if (targetDecision !== "go") {
			errors.push(`${evidence}: go decision requires ${target} backup target`);
		}
	}
}

function validateStatusSummary() {
	const rows = [...tableRowsAfter("## Backup Targets"), ...tableRowsAfter("## Restore Evidence")];
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
	if (!validStatuses.includes(status)) errors.push(`${label}: unsupported backup/restore status ${status}`);
	if (!validDecisions.includes(decision)) errors.push(`${label}: unsupported backup/restore decision ${decision}`);
	if (decision === "go" && !["passed", "accepted"].includes(status)) {
		errors.push(`${label}: go decision requires passed or accepted status`);
	}
	if (["pending", "blocking", "failed"].includes(status) && decision !== "no-go") {
		errors.push(`${label}: ${status} status requires no-go decision`);
	}
}

function hasConcreteEvidence(value) {
	const normalized = (value ?? "").toLowerCase();
	return /(artifact|checksum|digest|log|report|result|snapshot|command output|record|link|url|metric|measurement|timestamp|duration)/.test(normalized);
}

function strictBlockers() {
	const counts = { pending: 0, blocking: 0, failed: 0, "no-go": 0 };
	for (const row of [...tableRowsAfter("## Backup Targets"), ...tableRowsAfter("## Restore Evidence")]) {
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
	console.error("Backup restore checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Backup restore: ok (${requiredTargets.length} targets, ${requiredEvidence.length} evidence rows)`,
);
