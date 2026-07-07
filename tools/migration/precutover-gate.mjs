#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { optionValue } from "./cli-options.mjs";

const args = process.argv.slice(2);

if (args.includes("--help")) {
	console.log("Usage: node tools/migration/precutover-gate.mjs --env <env> --reconciliation-report <path> [--verbose]");
	process.exit(0);
}

const env = optionValue(args, "--env");
const reconciliationReport = optionValue(args, "--reconciliation-report");
const verbose = args.includes("--verbose");
const failures = [];
const maxFailureLines = 25;

function command(label, script, scriptArgs = []) {
	return {
		label,
		args: [script, ...scriptArgs],
	};
}

const commands = [
	command("migration artifacts", "tools/migration/check-artifacts.mjs"),
	command("blueprint", "tools/migration/blueprint.mjs"),
	command("runbook", "tools/migration/runbook.mjs"),
	command("tooling", "tools/migration/tooling.mjs"),
	command("repository controls", "tools/migration/repository-controls.mjs"),
	command("readiness report", "tools/migration/readiness-report.mjs", ["--strict"]),
	command("cutover evidence packet", "tools/migration/cutover-evidence-packet.mjs", ["--strict"]),
	command("live evidence schema", "tools/migration/live-evidence-schema.mjs"),
	command("live evidence instances", "tools/migration/live-evidence-instances.mjs", ["--strict"]),
	command("completion audit", "tools/migration/completion-audit.mjs", ["--strict"]),
	command("execution backlog", "tools/migration/execution-backlog.mjs", ["--strict"]),
	command("status consistency", "tools/migration/status-consistency.mjs"),
	command("parity", "tools/migration/parity.mjs", ["--strict"]),
	command("owner sign-offs", "tools/migration/owner-signoffs.mjs", ["--strict"]),
	command("cutover checklist", "tools/migration/cutover-checklist.mjs", ["--strict"]),
	command("rehearsals", "tools/migration/rehearsals.mjs", ["--strict"]),
	command("snapshots", "tools/migration/snapshots.mjs", ["--strict"]),
	command("release freeze", "tools/migration/release-freeze.mjs", ["--strict"]),
	command("backup restore", "tools/migration/backup-restore.mjs", ["--strict"]),
	command("communication", "tools/migration/communication.mjs", ["--strict"]),
	command("observability", "tools/migration/observability.mjs", ["--strict"]),
	command("smoke tests", "tools/migration/smoke-tests.mjs", ["--strict"]),
	command("rollback report", "tools/migration/rollback-report.mjs", ["--strict"]),
	command("rejects", "tools/migration/rejects.mjs", ["--strict"]),
	command("migration inventory", "tools/migration/inventory.mjs"),
	command("data decisions", "tools/migration/data-map.mjs", ["--strict"]),
	command("secret decisions", "tools/migration/secret-map.mjs", ["--strict"]),
	command("job decisions", "tools/migration/job-map.mjs", ["--strict"]),
	command("resource decisions", "tools/migration/resource-map.mjs", ["--strict"]),
	command("target structure", "tools/migration/target-structure.mjs", ["--strict"]),
	command("codegen", "tools/migration/codegen.mjs", ["--strict"]),
	command("supply chain", "tools/migration/supply-chain.mjs", ["--strict"]),
	command("runtime foundation", "tools/migration/runtime-foundation.mjs", ["--strict"]),
	command("phase ledger", "tools/migration/phase-ledger.mjs", ["--strict"]),
	command("domain ledger", "tools/migration/domain-ledger.mjs", ["--strict"]),
	command("domain DoD", "tools/migration/domain-dod.mjs", ["--strict"]),
	command("risk decisions", "tools/migration/risk-register.mjs", ["--strict"]),
	command("gate evidence", "tools/migration/gate-evidence.mjs", ["--strict"]),
];

if (!env) failures.push("--env is required");
if (!reconciliationReport) failures.push("--reconciliation-report is required");
if (env && reconciliationReport) {
	commands.push(
		command("reconciliation", "tools/migration/reconcile.mjs", [
			"--env",
			env,
			"--report",
			reconciliationReport,
		]),
	);
}

for (const item of commands) {
	const result = spawnSync(process.execPath, item.args, {
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
	});

	if (result.status === 0) {
		const line = result.stdout.trim().split("\n").at(-1);
		console.log(`ok: ${item.label}${line ? ` (${line})` : ""}`);
		continue;
	}

	failures.push(`${item.label} failed`);
	const output = `${result.stdout}${result.stderr}`.trim();
	if (output) {
		const lines = output.split("\n");
		const visibleLines = verbose ? lines : lines.slice(0, maxFailureLines);
		for (const line of visibleLines) {
			console.error(`${item.label}: ${line}`);
		}
		if (!verbose && lines.length > maxFailureLines) {
			console.error(
				`${item.label}: ... ${lines.length - maxFailureLines} more lines omitted; rerun with --verbose for full output`,
			);
		}
	}
}

if (failures.length > 0) {
	console.error("Migration precutover gate failed:");
	for (const failure of failures) console.error(`- ${failure}`);
	process.exit(1);
}

console.log(`Migration precutover gate: go (${env}, ${reconciliationReport})`);
