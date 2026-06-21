#!/usr/bin/env node
import { spawnSync } from "node:child_process";

const args = process.argv.slice(2);
const verbose = args.includes("--verbose");
const maxFailureLines = 25;
const failures = [];

const commands = [
	["migration artifacts", "tools/migration/check-artifacts.mjs", []],
	["blueprint", "tools/migration/blueprint.mjs", []],
	["runbook", "tools/migration/runbook.mjs", []],
	["tooling", "tools/migration/tooling.mjs", []],
	["repository controls", "tools/migration/repository-controls.mjs", []],
	["readiness report", "tools/migration/readiness-report.mjs", ["--strict"]],
	["cutover evidence packet", "tools/migration/cutover-evidence-packet.mjs", ["--strict"]],
	["live evidence schema", "tools/migration/live-evidence-schema.mjs", []],
	["live evidence instances", "tools/migration/live-evidence-instances.mjs", ["--strict"]],
	["completion audit", "tools/migration/completion-audit.mjs", ["--strict"]],
	["execution backlog", "tools/migration/execution-backlog.mjs", ["--strict"]],
	["status consistency", "tools/migration/status-consistency.mjs", []],
	["cutover journal", "tools/migration/cutover-journal.mjs", ["--strict"]],
	["rollback report", "tools/migration/rollback-report.mjs", ["--strict"]],
	["post-migration audit", "tools/migration/post-migration-audit.mjs", ["--strict"]],
	["decommission", "tools/migration/decommission.mjs", ["--strict"]],
	["target structure", "tools/migration/target-structure.mjs", ["--strict"]],
	["codegen", "tools/migration/codegen.mjs", ["--strict"]],
	["supply chain", "tools/migration/supply-chain.mjs", ["--strict"]],
	["runtime foundation", "tools/migration/runtime-foundation.mjs", ["--strict"]],
	["phase ledger", "tools/migration/phase-ledger.mjs", ["--strict"]],
	["domain ledger", "tools/migration/domain-ledger.mjs", ["--strict"]],
	["domain DoD", "tools/migration/domain-dod.mjs", ["--strict"]],
	["gate evidence", "tools/migration/gate-evidence.mjs", ["--strict"]],
];

for (const [label, script, scriptArgs] of commands) {
	const result = spawnSync(process.execPath, [script, ...scriptArgs], {
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
	});

	if (result.status === 0) {
		const line = result.stdout.trim().split("\n").at(-1);
		console.log(`ok: ${label}${line ? ` (${line})` : ""}`);
		continue;
	}

	failures.push(`${label} failed`);
	const output = `${result.stdout}${result.stderr}`.trim();
	if (!output) continue;
	const lines = output.split("\n");
	const visibleLines = verbose ? lines : lines.slice(0, maxFailureLines);
	for (const line of visibleLines) console.error(`${label}: ${line}`);
	if (!verbose && lines.length > maxFailureLines) {
		console.error(
			`${label}: ... ${lines.length - maxFailureLines} more lines omitted; rerun with --verbose for full output`,
		);
	}
}

if (failures.length > 0) {
	console.error("Migration postcutover gate failed:");
	for (const failure of failures) console.error(`- ${failure}`);
	process.exit(1);
}

console.log("Migration postcutover gate: go");
