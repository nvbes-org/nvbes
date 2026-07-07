#!/usr/bin/env node
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const strict = args.includes("--strict");
const path = "docs/migration/snapshot-manifest.md";
const content = readFileSync(path, "utf8");

const requiredHeadings = [
	"# Snapshot Manifest",
	"## Status",
	"## Rules",
	"## Source Snapshots",
	"## Target Versions",
	"## Restore Evidence",
	"## Decision",
];

const requiredSnapshotColumns = [
	"Snapshot",
	"Environment",
	"Source",
	"Created At",
	"Owner",
	"Checksum",
	"Storage Proof",
	"Retention",
	"Status",
];

const requiredTargetColumns = [
	"Run",
	"Commit",
	"Images",
	"SQL Migrations",
	"Contracts",
	"Artifact Check",
	"Status",
];

const requiredRestoreColumns = [
	"Snapshot",
	"Restore Command",
	"Duration",
	"Environment",
	"Evidence",
	"Decision",
];

const requiredRules = [
	"immutable source snapshot",
	"checksum or",
	"storage proof",
	"restore evidence",
	"retention",
];

const validStatuses = ["pending", "passed", "accepted", "failed", "blocking", "no-go"];
const errors = [];

for (const heading of requiredHeadings) {
	if (!content.includes(heading)) errors.push(`missing heading: ${heading}`);
}

for (const column of requiredSnapshotColumns) {
	if (!content.includes(column)) errors.push(`missing source snapshot column: ${column}`);
}

for (const column of requiredTargetColumns) {
	if (!content.includes(column)) errors.push(`missing target version column: ${column}`);
}

for (const column of requiredRestoreColumns) {
	if (!content.includes(column)) errors.push(`missing restore evidence column: ${column}`);
}

for (const rule of requiredRules) {
	if (!content.includes(rule)) errors.push(`missing snapshot rule: ${rule}`);
}

validateTable("## Source Snapshots", requiredSnapshotColumns);
validateTable("## Target Versions", requiredTargetColumns);
validateTable("## Restore Evidence", requiredRestoreColumns);
validateStatusSummary();

const sourceSnapshotRows = tableRowsAfter("## Source Snapshots");
const targetVersionRows = tableRowsAfter("## Target Versions");
const restoreRows = tableRowsAfter("## Restore Evidence");

for (const row of sourceSnapshotRows) {
	validateSourceSnapshotRow(row);
}
for (const row of targetVersionRows) {
	validateTargetVersionRow(row);
}
for (const row of restoreRows) {
	validateRestoreRow(row);
}

if (!content.includes("Current decision: no-go.")) errors.push("missing current no-go decision");
if (!content.includes("No rehearsal, rollback or cutover can be approved")) {
	errors.push("missing explicit snapshot approval blocker");
}

if (strict) {
	const blockers = strictBlockers();
	for (const [blocker, count] of Object.entries(blockers)) {
		if (count > 0) errors.push(`${count} ${blocker} snapshot marker(s) remain`);
	}
}

function tableRowsAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Snapshot |") && !line.includes("Run |"))
		.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()));
}

function tableHeaderAfter(heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	const header = section.split("\n").find((line) => line.startsWith("|") && !line.includes("---"));
	return header ? header.split("|").slice(1, -1).map((cell) => cell.trim()) : [];
}

function validateTable(heading, expectedColumns) {
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
}

function validateSourceSnapshotRow(row) {
	const [snapshot, environment, source, createdAt, owner, checksum, storageProof, retention, status] = row;
	if (!validStatuses.includes(status)) {
		errors.push(`${snapshot}: unsupported snapshot status ${status}`);
	}
	if (["failed", "blocking", "no-go"].includes(status) && isEmptyMarker(owner)) {
		errors.push(`${snapshot}: ${status} snapshot requires owner`);
	}
	if (["passed", "accepted"].includes(status)) {
		for (const [field, value] of [["environment", environment], ["source", source], ["created at", createdAt], ["owner", owner], ["retention", retention]]) {
			if (isEmptyMarker(value)) errors.push(`${snapshot}: ${status} snapshot requires ${field}`);
		}
		if (Number.isNaN(Date.parse(createdAt))) errors.push(`${snapshot}: ${status} snapshot requires parseable created at`);
		if (isEmptyMarker(checksum) && isEmptyMarker(storageProof)) {
			errors.push(`${snapshot}: ${status} snapshot requires checksum or storage proof`);
		}
		if (!hasConcreteEvidence(checksum) && !hasConcreteEvidence(storageProof)) {
			errors.push(`${snapshot}: ${status} snapshot requires concrete checksum or storage proof`);
		}
	}
}

function validateTargetVersionRow(row) {
	const [run, commit, images, migrations, contracts, artifactCheck, status] = row;
	if (!validStatuses.includes(status)) {
		errors.push(`${run}: unsupported target version status ${status}`);
	}
	if (["passed", "accepted"].includes(status)) {
		for (const [field, value] of [["commit", commit], ["images", images], ["SQL migrations", migrations], ["contracts", contracts], ["artifact check", artifactCheck]]) {
			if (isEmptyMarker(value)) errors.push(`${run}: ${status} target version requires ${field}`);
		}
		for (const [field, value] of [["commit", commit], ["images", images], ["artifact check", artifactCheck]]) {
			if (!hasConcreteEvidence(value)) errors.push(`${run}: ${status} target version requires concrete ${field}`);
		}
	}
}

function validateRestoreRow(row) {
	const [snapshot, command, duration, environment, evidence, decision] = row;
	if (!["go", "no-go"].includes(decision)) errors.push(`${snapshot}: unsupported restore decision ${decision}`);
	if (decision === "no-go" && !row.some((value) => isEmptyMarker(value))) {
		errors.push(`${snapshot}: no-go restore decision must identify missing evidence`);
	}
	if (decision === "go") {
		for (const [field, value] of [["snapshot", snapshot], ["restore command", command], ["duration", duration], ["environment", environment], ["evidence", evidence]]) {
			if (isEmptyMarker(value)) errors.push(`${snapshot}: go restore decision requires ${field}`);
		}
		if (!hasConcreteEvidence(evidence)) errors.push(`${snapshot}: go restore decision requires concrete evidence`);
		validateRestoreDependencies(snapshot);
	}
}

function validateRestoreDependencies(snapshot) {
	for (const row of sourceSnapshotRows) {
		if (!["passed", "accepted"].includes(row.at(-1))) {
			errors.push(`${snapshot}: go restore decision requires ${row[0]} source snapshot row`);
		}
	}
	for (const row of targetVersionRows) {
		if (!["passed", "accepted"].includes(row.at(-1))) {
			errors.push(`${snapshot}: go restore decision requires ${row[0]} target version row`);
		}
	}
}

function validateStatusSummary() {
	const sourceSnapshots = tableRowsAfter("## Source Snapshots");
	const targetVersions = tableRowsAfter("## Target Versions");
	const restoreRows = tableRowsAfter("## Restore Evidence");
	const sourceStatuses = countValues(sourceSnapshots.map((row) => row.at(-1)), validStatuses);
	const targetStatuses = countValues(targetVersions.map((row) => row.at(-1)), validStatuses);
	const restoreDecisions = countValues(restoreRows.map((row) => row.at(-1)), ["go", "no-go"]);
	const expected = {
		total_source_snapshot_rows: sourceSnapshots.length,
		source_snapshot_no_go_rows: sourceStatuses["no-go"],
		source_snapshot_pending_rows: sourceStatuses.pending,
		source_snapshot_passed_rows: sourceStatuses.passed,
		source_snapshot_accepted_rows: sourceStatuses.accepted,
		source_snapshot_failed_rows: sourceStatuses.failed,
		source_snapshot_blocking_rows: sourceStatuses.blocking,
		total_target_version_rows: targetVersions.length,
		target_version_no_go_rows: targetStatuses["no-go"],
		target_version_pending_rows: targetStatuses.pending,
		target_version_passed_rows: targetStatuses.passed,
		target_version_accepted_rows: targetStatuses.accepted,
		target_version_failed_rows: targetStatuses.failed,
		target_version_blocking_rows: targetStatuses.blocking,
		total_restore_rows: restoreRows.length,
		restore_go_decisions: restoreDecisions.go,
		restore_no_go_decisions: restoreDecisions["no-go"],
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
	return ["", "pending", "none", "no-go"].includes(value ?? "");
}

function hasConcreteEvidence(value) {
	return /(sha|checksum|digest|artifact|manifest|report|result|log|snapshot|storage|url|link|command output|record|version)/i.test(value ?? "");
}

function strictBlockers() {
	const counts = { pending: 0, blocking: 0, failed: 0, "no-go": 0 };
	for (const row of [...tableRowsAfter("## Source Snapshots"), ...tableRowsAfter("## Target Versions")]) {
		const status = row.at(-1);
		if (status === "pending") counts.pending += 1;
		if (status === "blocking") counts.blocking += 1;
		if (status === "failed") counts.failed += 1;
		if (status === "no-go") counts["no-go"] += 1;
	}
	for (const row of tableRowsAfter("## Restore Evidence")) {
		if (row.at(-1) === "no-go") counts["no-go"] += 1;
	}
	return counts;
}

if (errors.length > 0) {
	console.error("Snapshot checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Snapshots: ok (${requiredSnapshotColumns.length} snapshot columns, ${requiredRestoreColumns.length} restore columns)`,
);
