import { strictCommandFor } from "./cutover-evidence-packet.requirements.mjs";
import { validateProofCommand } from "./execution-backlog.proof.mjs";
import { buildPreparationCommands, commandCoverageFor } from "./live-evidence-commands.mjs";
import { requiredEvidenceFor } from "./live-evidence.rules.mjs";

export function validatePacket(packet, context) {
	const errors = [];
	const { strict, jsonPath, requirements, scripts, completion, backlog, liveBlockingReasons, isLiveBlocking } = context;
	validateGeneration(packet, jsonPath, errors);
	validateStatusShape(packet, jsonPath, errors);
	validateRequirementRows(packet, jsonPath, requirements, liveBlockingReasons, errors);
	validateStatusConsistency(packet, jsonPath, completion, backlog, isLiveBlocking, errors);
	validateTaskProofs(packet, scripts, errors);
	if (strict && packet.status.decision !== "go") errors.push(`${jsonPath}: cutover evidence decision is ${packet.status.decision}`);
	return errors;
}

function validateGeneration(packet, jsonPath, errors) {
	if (packet.schema_version !== 1) errors.push(`${jsonPath}: schema_version must be 1`);
	if (packet.generation?.command !== "tools/migration/cutover-evidence-packet.mjs --write") errors.push(`${jsonPath}: generation.command is invalid`);
	if (packet.generation?.strict_command !== "tools/migration/cutover-evidence-packet.mjs --strict") errors.push(`${jsonPath}: generation.strict_command is invalid`);
}

function validateStatusShape(packet, jsonPath, errors) {
	const status = packet.status ?? {};
	if (!["go", "no-go"].includes(status.readiness)) errors.push(`${jsonPath}: status.readiness must be go or no-go`);
	if (typeof status.repository_ready !== "boolean") errors.push(`${jsonPath}: status.repository_ready must be boolean`);
	if (!Number.isInteger(status.live_blocking_items) || status.live_blocking_items < 0) errors.push(`${jsonPath}: status.live_blocking_items must be a non-negative integer`);
	if (!["go", "no-go"].includes(status.completion)) errors.push(`${jsonPath}: status.completion must be go or no-go`);
	if (!Number.isInteger(status.backlog_open_tasks) || status.backlog_open_tasks < 0) errors.push(`${jsonPath}: status.backlog_open_tasks must be a non-negative integer`);
	if (!Number.isInteger(status.backlog_blocking_items) || status.backlog_blocking_items < 0) errors.push(`${jsonPath}: status.backlog_blocking_items must be a non-negative integer`);
	if (!["go", "no-go"].includes(status.decision)) errors.push(`${jsonPath}: status.decision must be go or no-go`);
}

function validateRequirementRows(packet, jsonPath, requirements, liveBlockingReasons, errors) {
	if (!Array.isArray(packet.requirements) || packet.requirements.length !== requirements.length) errors.push(`${jsonPath}: requirements are incomplete`);
	const expectedById = new Map(requirements.map((requirement) => [requirement.id, requirement]));
	const seen = new Set();
	for (const row of packet.requirements ?? []) validateRequirementRow(row, jsonPath, expectedById, seen, liveBlockingReasons, errors);
	for (const requirement of requirements) if (!seen.has(requirement.id)) errors.push(`${jsonPath}: missing requirement ${requirement.id}`);
}

function validateRequirementRow(row, jsonPath, expectedById, seen, liveBlockingReasons, errors) {
	if (seen.has(row.id)) errors.push(`${jsonPath}: duplicate requirement ${row.id}`);
	seen.add(row.id);
	const expected = expectedById.get(row.id);
	if (!expected) return errors.push(`${jsonPath}: unexpected requirement ${row.id}`);
	if (row.scope !== expected.scope) errors.push(`${row.id}: scope must match static requirement`);
	if (row.live_evidence !== expected.live_evidence) errors.push(`${row.id}: live_evidence must match static requirement`);
	validateStringList(row.id, "files", row.files, expected.files, errors);
	validateStringList(row.id, "commands", row.commands, expected.commands, errors);
	validateStringList(
		row.id,
		"preparation_commands",
		row.preparation_commands,
		buildPreparationCommands([{ ...expected, expected_evidence: requiredEvidenceFor(expected.id) }]),
		errors,
	);
	if (row.strict_command !== strictCommandFor(expected)) errors.push(`${row.id}: strict_command must match static requirement`);
	const commandCoverage = commandCoverageFor(row.strict_command, row.preparation_commands ?? []);
	validateStringList(row.id, "strict_command_segments", row.strict_command_segments, commandCoverage.strict_command_segments, errors);
	validateStringList(row.id, "evidence_command_segments", row.evidence_command_segments, commandCoverage.evidence_command_segments, errors);
	validateStringList(
		row.id,
		"uncovered_strict_command_segments",
		row.uncovered_strict_command_segments,
		commandCoverage.uncovered_strict_command_segments,
		errors,
	);
	if (!Array.isArray(row.missing_files)) errors.push(`${row.id}: missing_files must be an array`);
	if (!Array.isArray(row.missing_commands)) errors.push(`${row.id}: missing_commands must be an array`);
	if (row.repository_ready !== ((row.missing_files?.length ?? 1) === 0 && (row.missing_commands?.length ?? 1) === 0)) errors.push(`${row.id}: repository_ready must match missing files and commands`);
	const reasons = liveBlockingReasons(row.id, row.repository_ready, row.current);
	if ((row.live_blocking_reasons ?? []).join("\n") !== reasons.join("\n")) errors.push(`${row.id}: live_blocking_reasons must match current state`);
}

function validateTaskProofs(packet, scripts, errors) {
	for (const row of packet.requirements ?? []) {
		if (!row.id) errors.push("requirement id is required");
		if (!row.scope) errors.push(`${row.id}: scope is required`);
		if (!row.live_evidence) errors.push(`${row.id}: live_evidence is required`);
		if (!row.current || typeof row.current !== "object") errors.push(`${row.id}: current is required`);
		if (typeof row.current?.phase !== "string") errors.push(`${row.id}: current.phase is required`);
		if (typeof row.current?.gate !== "string") errors.push(`${row.id}: current.gate is required`);
		if (!Array.isArray(row.live_blocking_reasons)) errors.push(`${row.id}: live_blocking_reasons must be an array`);
		if (!Array.isArray(row.preparation_commands) || row.preparation_commands.length === 0) {
			errors.push(`${row.id}: preparation_commands are required`);
		}
		if (!row.strict_command) errors.push(`${row.id}: strict_command is required`);
		else errors.push(...validateProofCommand({ id: row.id, proof: row.strict_command }, scripts));
		for (const missing of row.missing_files ?? []) errors.push(`${row.id}: missing file ${missing}`);
		for (const missing of row.missing_commands ?? []) errors.push(`${row.id}: missing command ${missing}`);
	}
}

function validateStatusConsistency(packet, jsonPath, completion, backlog, isLiveBlocking, errors) {
	const rows = packet.requirements ?? [];
	const liveBlocking = rows.filter(isLiveBlocking);
	const repositoryReady = rows.every((row) => row.repository_ready);
	if (packet.status.repository_ready !== repositoryReady) errors.push(`${jsonPath}: repository_ready must match requirement rows`);
	if (packet.status.live_blocking_items !== liveBlocking.length) errors.push(`${jsonPath}: live_blocking_items must match blocking requirement rows`);
	const completionReady = completion?.status?.objective === "complete" && (completion?.status?.incomplete ?? 1) === 0;
	const backlogClear = (backlog?.status?.open_tasks ?? 1) === 0 && (backlog?.status?.blocking_items ?? 1) === 0;
	if (packet.status.completion !== (completionReady ? "go" : "no-go")) errors.push(`${jsonPath}: completion status must match completion audit`);
	if (packet.status.backlog_open_tasks !== (backlog?.status?.open_tasks ?? null)) errors.push(`${jsonPath}: backlog_open_tasks must match execution backlog`);
	if (packet.status.backlog_blocking_items !== (backlog?.status?.blocking_items ?? null)) errors.push(`${jsonPath}: backlog_blocking_items must match execution backlog`);
	if (packet.status.decision === "go") {
		if (packet.status.readiness !== "go") errors.push(`${jsonPath}: go decision requires readiness go`);
		if (!repositoryReady) errors.push(`${jsonPath}: go decision requires repository readiness`);
		if (liveBlocking.length > 0) errors.push(`${jsonPath}: go decision requires zero live blockers`);
		if (rows.some((row) => (row.uncovered_strict_command_segments ?? []).length > 0)) {
			errors.push(`${jsonPath}: go decision requires complete strict command coverage`);
		}
		if (!completionReady) errors.push(`${jsonPath}: go decision requires completion audit complete`);
		if (!backlogClear) errors.push(`${jsonPath}: go decision requires zero backlog tasks and blockers`);
	}
	if (packet.status.decision === "no-go" && packet.status.readiness === "go" && repositoryReady && liveBlocking.length === 0 && completionReady && backlogClear) errors.push(`${jsonPath}: no-go decision requires a readiness, repository, live evidence, completion or backlog blocker`);
}

function validateStringList(id, field, actual, expected, errors) {
	if (!Array.isArray(actual)) return errors.push(`${id}: ${field} must be an array`);
	if (actual.length !== expected.length || actual.some((value, index) => value !== expected[index])) errors.push(`${id}: ${field} must match static requirement`);
}
