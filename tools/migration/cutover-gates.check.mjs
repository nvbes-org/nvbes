#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";

const errors = [];

const gateChecks = [
	{
		path: "tools/migration/precutover-gate.mjs",
		name: "precutover",
		requiredSubstrings: [
			'Usage: node tools/migration/precutover-gate.mjs --env <env> --reconciliation-report <path> [--verbose]',
			'failures.push("--env is required")',
			'failures.push("--reconciliation-report is required")',
			'command("readiness report", "tools/migration/readiness-report.mjs", ["--strict"])',
			'command("cutover evidence packet", "tools/migration/cutover-evidence-packet.mjs", ["--strict"])',
			'command("live evidence schema", "tools/migration/live-evidence-schema.mjs")',
			'command("live evidence instances", "tools/migration/live-evidence-instances.mjs", ["--strict"])',
			'command("completion audit", "tools/migration/completion-audit.mjs", ["--strict"])',
			'command("execution backlog", "tools/migration/execution-backlog.mjs", ["--strict"])',
			'command("owner sign-offs", "tools/migration/owner-signoffs.mjs", ["--strict"])',
			'command("status consistency", "tools/migration/status-consistency.mjs")',
			'command("cutover checklist", "tools/migration/cutover-checklist.mjs", ["--strict"])',
			'command("rehearsals", "tools/migration/rehearsals.mjs", ["--strict"])',
			'command("backup restore", "tools/migration/backup-restore.mjs", ["--strict"])',
			'command("rollback report", "tools/migration/rollback-report.mjs", ["--strict"])',
			'command("phase ledger", "tools/migration/phase-ledger.mjs", ["--strict"])',
			'command("gate evidence", "tools/migration/gate-evidence.mjs", ["--strict"])',
			'command("reconciliation", "tools/migration/reconcile.mjs"',
			'"--env"',
			'"--report"',
		],
	},
	{
		path: "tools/migration/postcutover-gate.mjs",
		name: "postcutover",
		requiredSubstrings: [
			'["readiness report", "tools/migration/readiness-report.mjs", ["--strict"]]',
			'["cutover evidence packet", "tools/migration/cutover-evidence-packet.mjs", ["--strict"]]',
			'["live evidence schema", "tools/migration/live-evidence-schema.mjs", []]',
			'["live evidence instances", "tools/migration/live-evidence-instances.mjs", ["--strict"]]',
			'["completion audit", "tools/migration/completion-audit.mjs", ["--strict"]]',
			'["execution backlog", "tools/migration/execution-backlog.mjs", ["--strict"]]',
			'["status consistency", "tools/migration/status-consistency.mjs", []]',
			'["cutover journal", "tools/migration/cutover-journal.mjs", ["--strict"]]',
			'["rollback report", "tools/migration/rollback-report.mjs", ["--strict"]]',
			'["post-migration audit", "tools/migration/post-migration-audit.mjs", ["--strict"]]',
			'["decommission", "tools/migration/decommission.mjs", ["--strict"]]',
			'["phase ledger", "tools/migration/phase-ledger.mjs", ["--strict"]]',
			'["gate evidence", "tools/migration/gate-evidence.mjs", ["--strict"]]',
		],
	},
];

for (const gate of gateChecks) {
	if (!existsSync(gate.path)) {
		errors.push(`${gate.path}: missing`);
		continue;
	}
	const content = readFileSync(gate.path, "utf8");
	for (const required of gate.requiredSubstrings) {
		if (!content.includes(required)) errors.push(`${gate.name}: missing ${required}`);
	}
	if (!content.includes("process.exit(1)")) errors.push(`${gate.name}: failures must exit non-zero`);
	if (!content.includes("maxFailureLines")) errors.push(`${gate.name}: failure output must be bounded`);
}

if (errors.length > 0) {
	console.error("Migration cutover gate composition checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log("Migration cutover gate composition: ok (2 gates)");
