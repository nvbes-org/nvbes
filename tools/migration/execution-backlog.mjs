#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readinessBlockingDetails } from "./execution-backlog.blockers.mjs";
import { completionBlockingDetails } from "./execution-backlog.completion.mjs";
import { loadBacklogInputs } from "./execution-backlog.inputs.mjs";
import { preparationCommandsForRequirement } from "./execution-backlog.live-evidence.mjs";
import { readPackageScripts } from "./execution-backlog.proof.mjs";
import { validateBacklog } from "./execution-backlog.validation.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const jsonPath = "docs/migration/execution-backlog.generated.json";
const markdownPath = "docs/migration/execution-backlog.md";
const errors = [];
const packageScripts = readPackageScripts(errors);
let inputState;

const ownerByArea = {
	data: "Data lead",
	secrets: "Security lead",
	jobs: "Infra lead",
	resources: "Infra lead",
	target_structure: "Migration lead",
	codegen: "Platform lead",
	supply_chain: "Security lead",
	runtimes: "Platform lead",
	phases: "Migration lead",
	domains: "Migration lead",
	domain_dod: "Migration lead",
	risks: "Migration lead",
	gates: "Migration lead",
	live_evidence: "Migration lead",
	communication: "Support lead",
	cutover_checklist: "Migration lead",
	cutover_journal: "Migration lead",
	observability: "Infra lead",
	owner_signoffs: "Migration lead",
	reconciliation_template: "Data lead",
	reconciliation_report: "Data lead",
	rehearsals: "Data lead",
	rejects: "Data lead",
	release_freeze: "Migration lead",
	snapshots: "Data lead",
};

const ownerByLiveRequirement = {
	"g4-frontend-signoff": "Product leads",
	"g5-infra-signoff": "Infra lead",
	"p11-rehearsals": "Data lead",
	"p12-cutover": "Migration lead",
	"p13-decommission": "Infra lead",
	"final-reconciliation": "Data lead",
};

const commandByArea = {
	data: "node tools/migration/data-map.mjs --strict",
	secrets: "node tools/migration/secret-map.mjs --strict",
	jobs: "node tools/migration/job-map.mjs --strict",
	resources: "node tools/migration/resource-map.mjs --strict",
	target_structure: "node tools/migration/target-structure.mjs --strict",
	codegen: "node tools/migration/codegen.mjs --strict",
	supply_chain: "node tools/migration/supply-chain.mjs --strict",
	runtimes: "node tools/migration/runtime-foundation.mjs --strict",
	phases: "node tools/migration/phase-ledger.mjs --strict",
	domains: "node tools/migration/domain-ledger.mjs --strict",
	domain_dod: "node tools/migration/domain-dod.mjs --strict",
	risks: "node tools/migration/risk-register.mjs --strict",
	gates: "node tools/migration/gate-evidence.mjs --strict",
	live_evidence: "node tools/migration/live-evidence-instances.mjs --strict",
	communication: "pnpm check:migration-communication -- --strict",
	cutover_checklist: "pnpm check:migration-cutover-checklist -- --strict",
	cutover_journal: "pnpm check:migration-cutover-journal -- --strict",
	observability: "pnpm check:migration-observability -- --strict",
	owner_signoffs: "pnpm check:migration-owner-signoffs -- --strict",
	reconciliation_template: "node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json",
	reconciliation_report: "pnpm check:migration-reconciliation-report -- --strict",
	rehearsals: "pnpm check:migration-rehearsals -- --strict",
	rejects: "pnpm check:migration-rejects -- --strict",
	release_freeze: "pnpm check:migration-release-freeze -- --strict",
	snapshots: "pnpm check:migration-snapshots -- --strict",
};

function buildTasks(readiness, completion, liveEvidence, blockerSources) {
	const tasks = [];
	for (const blocker of readiness?.blockers ?? []) {
		const blockingDetails = readinessBlockingDetails(blocker, { liveEvidence, ...blockerSources });
		tasks.push({
			id: `resolve-${blocker.area}`,
			owner_role: ownerByArea[blocker.area] ?? "Migration lead",
			source: blocker.source,
			blocking_items: blocker.blocking_items,
			blocking_details: blockingDetails,
			next_action: `resolve and sign ${blocker.area} decisions`,
			proof: commandByArea[blocker.area] ?? "pnpm check:migration-precutover",
			status: blocker.blocking_items === 0 ? "complete" : "blocked",
		});
	}
	for (const item of completion?.requirements ?? []) {
		if (item.status === "complete" || item.status === "control-active") continue;
		tasks.push({
			id: `complete-${item.id}`,
			owner_role: "Migration lead",
			source: item.source,
			blocking_items: 1,
			blocking_details: completionBlockingDetails(item, blockerSources),
			next_action: item.requirement,
			proof: item.proof,
			status: "blocked",
		});
	}
	for (const requirement of liveEvidence?.requirements ?? []) {
		if (requirement.ready) continue;
		const missing = requirement.missing_evidence ?? [];
		tasks.push({
			id: `attach-live-evidence-${requirement.id}`,
			owner_role: ownerByLiveRequirement[requirement.id] ?? "Migration lead",
			source: `live-evidence:${requirement.scope}`,
			blocking_items: Math.max(1, missing.length),
			blocking_details: missing.length > 0 ? missing : [`${requirement.id} live evidence missing`],
			next_action: `attach accepted live evidence for ${missing.join(", ")}`,
			preparation_commands: preparationCommandsForRequirement(requirement),
			proof: requirement.strict_command
				? `${requirement.strict_command} && pnpm check:migration-live-evidence-instances -- --strict`
				: "node tools/migration/live-evidence-instances.mjs --strict",
			status: "blocked",
		});
	}
	return tasks.sort((a, b) => a.id.localeCompare(b.id));
}

function buildBacklog() {
	const { readiness, completion, liveEvidence, blockerSources } = loadBacklogInputs(errors);
	inputState = { readiness, completion, liveEvidence };
	const tasks = buildTasks(readiness, completion, liveEvidence, blockerSources);
	return {
		schema_version: 1,
		generation: {
			command: "node tools/migration/execution-backlog.mjs --write",
			strict_completion_command: "node tools/migration/execution-backlog.mjs --strict",
		},
		status: {
			open_tasks: tasks.filter((task) => task.status !== "complete").length,
			blocking_items: tasks.reduce((sum, task) => sum + task.blocking_items, 0),
			readiness: readiness?.status?.production_cutover ?? "unknown",
			completion: completion?.status?.objective ?? "unknown",
			live_evidence: liveEvidence?.status?.decision ?? "unknown",
			live_evidence_missing_requirements: liveEvidence?.status?.missing_requirements ?? null,
			live_evidence_missing_items: liveEvidence?.status?.missing_evidence_items ?? null,
		},
		tasks,
	};
}

function serializeJson(backlog) {
	return `${JSON.stringify(backlog, null, 2)}\n`;
}

function blockingDetailsCell(task) {
	return task.blocking_details.map((detail) => `\`${detail}\``).join("<br>");
}

function serializeMarkdown(backlog) {
	const lines = [
		"# Migration Execution Backlog",
		"",
		"## Status",
		"",
		`- open_tasks: ${backlog.status.open_tasks}`,
		`- blocking_items: ${backlog.status.blocking_items}`,
		`- readiness: ${backlog.status.readiness}`,
		`- completion: ${backlog.status.completion}`,
		`- live_evidence: ${backlog.status.live_evidence}`,
		`- live_evidence_missing_requirements: ${backlog.status.live_evidence_missing_requirements}`,
		`- live_evidence_missing_items: ${backlog.status.live_evidence_missing_items}`,
		"",
		"## Rules",
		"",
		"- `open_tasks` must equal generated tasks that are not complete.",
		"- `blocking_items` must equal the sum of task blocking counts.",
		"- Blocked tasks require at least one blocking item.",
		"- Complete tasks must have zero blocking items.",
		"- Task IDs must be unique and every task must carry owner and proof.",
		"- Markdown task rows must expose owner, source, next action, status, blocking count, blocking details and proof.",
		"- Proof commands must reference existing package scripts or migration tools.",
		"- Live evidence tasks must list generated preparation command templates.",
		"- Generation provenance must identify the write and strict check commands.",
		"",
		"## Tasks",
		"",
		"| ID | Owner | Source | Next action | Status | Blocking | Blocking details | Proof |",
		"|---|---|---|---|---|---:|---|---|",
	];
	for (const task of backlog.tasks) {
		lines.push(
			`| ${task.id} | ${task.owner_role} | ${task.source} | ${task.next_action} | ${task.status} | ${task.blocking_items} | ${blockingDetailsCell(task)} | \`${task.proof}\` |`,
		);
	}
	const liveTasks = backlog.tasks.filter((task) => task.preparation_commands?.length > 0);
	lines.push("", "## Live Evidence Preparation", "");
	for (const task of liveTasks) {
		lines.push(`### ${task.id}`, "", "```bash", ...task.preparation_commands, "```", "");
	}
	lines.push("", "## Regeneration", "", "```bash", "pnpm check:migration-execution-backlog", "node tools/migration/execution-backlog.mjs --write", "```", "");
	return lines.join("\n");
}

function validate(backlog) {
	errors.push(...validateBacklog(backlog, { strict, jsonPath, packageScripts, ...inputState }));
}

const backlog = buildBacklog();
const json = serializeJson(backlog);
const markdown = serializeMarkdown(backlog);

if (write) {
	mkdirSync(dirname(jsonPath), { recursive: true });
	writeFileSync(jsonPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration execution backlog written to ${markdownPath} and ${jsonPath}`);
	process.exit(0);
}

validate(backlog);

for (const [path, expected] of [[jsonPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing; run node tools/migration/execution-backlog.mjs --write`);
	} else if (readFileSync(path, "utf8") !== expected) {
		errors.push(`${path}: stale; run node tools/migration/execution-backlog.mjs --write`);
	}
}

if (errors.length > 0) {
	console.error("Migration execution backlog checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Migration execution backlog: ok (${backlog.status.open_tasks} open tasks, ${backlog.status.blocking_items} blocking items)`,
);
