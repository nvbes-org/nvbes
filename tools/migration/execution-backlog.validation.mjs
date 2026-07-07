import { validateProofCommand } from "./execution-backlog.proof.mjs";

export function validateBacklog(backlog, context) {
	const errors = [];
	const { strict, jsonPath, packageScripts, readiness, completion, liveEvidence } = context;
	if (backlog.schema_version !== 1) errors.push(`${jsonPath}: schema_version must be 1`);
	if (backlog.generation?.command !== "node tools/migration/execution-backlog.mjs --write") {
		errors.push(`${jsonPath}: generation.command is invalid`);
	}
	if (backlog.generation?.strict_completion_command !== "node tools/migration/execution-backlog.mjs --strict") {
		errors.push(`${jsonPath}: generation.strict_completion_command is invalid`);
	}
	if (!Array.isArray(backlog.tasks)) errors.push(`${jsonPath}: tasks must be an array`);
	validateStatusShape(backlog, jsonPath, errors);
	validateStatusConsistency(backlog, jsonPath, errors);
	validateTaskRows(backlog, packageScripts, errors);
	validateCoverage(backlog, readiness, completion, liveEvidence, errors);
	if (strict && backlog.status.open_tasks > 0) {
		errors.push(`${jsonPath}: ${backlog.status.open_tasks} open task(s) remain`);
	}
	return errors;
}

function validateStatusShape(backlog, jsonPath, errors) {
	const status = backlog.status ?? {};
	if (!Number.isInteger(status.open_tasks) || status.open_tasks < 0) errors.push(`${jsonPath}: status.open_tasks must be a non-negative integer`);
	if (!Number.isInteger(status.blocking_items) || status.blocking_items < 0) errors.push(`${jsonPath}: status.blocking_items must be a non-negative integer`);
	if (!["go", "no-go", "unknown"].includes(status.readiness)) errors.push(`${jsonPath}: status.readiness is invalid`);
	if (!["complete", "incomplete", "unknown"].includes(status.completion)) errors.push(`${jsonPath}: status.completion is invalid`);
	if (!["go", "no-go", "unknown"].includes(status.live_evidence)) errors.push(`${jsonPath}: status.live_evidence is invalid`);
	if (!Number.isInteger(status.live_evidence_missing_requirements) || status.live_evidence_missing_requirements < 0) {
		errors.push(`${jsonPath}: status.live_evidence_missing_requirements must be a non-negative integer`);
	}
	if (!Number.isInteger(status.live_evidence_missing_items) || status.live_evidence_missing_items < 0) {
		errors.push(`${jsonPath}: status.live_evidence_missing_items must be a non-negative integer`);
	}
}

function validateStatusConsistency(backlog, jsonPath, errors) {
	const tasks = Array.isArray(backlog.tasks) ? backlog.tasks : [];
	const openTasks = tasks.filter((task) => task.status !== "complete").length;
	const blockingItems = tasks.reduce((sum, task) => sum + (task.blocking_items ?? 0), 0);
	if (backlog.status.open_tasks !== openTasks) errors.push(`${jsonPath}: open_tasks must match task rows`);
	if (backlog.status.completion === "complete" && openTasks !== 0) errors.push(`${jsonPath}: complete status requires zero open tasks`);
	if (backlog.status.live_evidence === "go" && backlog.status.live_evidence_missing_requirements !== 0) {
		errors.push(`${jsonPath}: live evidence go requires zero missing requirements`);
	}
	if (backlog.status.live_evidence === "go" && backlog.status.live_evidence_missing_items !== 0) {
		errors.push(`${jsonPath}: live evidence go requires zero missing evidence items`);
	}
	if (backlog.status.blocking_items !== blockingItems) errors.push(`${jsonPath}: blocking_items must equal task blocking total`);
	const ids = new Set();
	for (const task of tasks) {
		if (ids.has(task.id)) errors.push(`${jsonPath}: duplicate task id ${task.id}`);
		ids.add(task.id);
	}
	for (let index = 1; index < tasks.length; index += 1) {
		if (tasks[index - 1].id > tasks[index].id) errors.push(`${jsonPath}: tasks must be sorted by id`);
	}
}

function validateTaskRows(backlog, packageScripts, errors) {
	for (const task of backlog.tasks ?? []) {
		if (!task.id) errors.push("task id is required");
		if (!task.owner_role) errors.push(`${task.id}: owner_role is required`);
		if (!task.source) errors.push(`${task.id}: source is required`);
		if (!task.next_action) errors.push(`${task.id}: next_action is required`);
		if (hasPlaceholder(task.source)) errors.push(`${task.id}: source must be concrete`);
		if (hasPlaceholder(task.next_action)) errors.push(`${task.id}: next_action must be concrete`);
		if (!task.proof) errors.push(`${task.id}: proof is required`);
		else errors.push(...validateProofCommand(task, packageScripts));
		if (task.id?.startsWith("attach-live-evidence-") && !task.preparation_commands?.length) {
			errors.push(`${task.id}: live evidence task requires preparation_commands`);
		}
		if (!["blocked", "complete"].includes(task.status)) errors.push(`${task.id}: unsupported status ${task.status}`);
		if (!Number.isInteger(task.blocking_items) || task.blocking_items < 0) {
			errors.push(`${task.id}: blocking_items must be a non-negative integer`);
		}
		if (!Array.isArray(task.blocking_details)) errors.push(`${task.id}: blocking_details must be an array`);
		else if (task.blocking_details.length !== task.blocking_items) {
			errors.push(`${task.id}: blocking_details must match blocking_items`);
		}
		for (const detail of task.blocking_details ?? []) {
			if (hasWeakGeneratedDetail(detail)) errors.push(`${task.id}: blocking_details must name concrete source items`);
		}
		if (task.status === "blocked" && task.blocking_items < 1) errors.push(`${task.id}: blocked task requires at least one blocking item`);
		if (task.status === "complete" && task.blocking_items !== 0) errors.push(`${task.id}: complete task requires zero blocking items`);
	}
}

function hasPlaceholder(value) {
	return ["", "pending", "none", "todo", "tbd", "unknown"].includes(String(value ?? "").trim().toLowerCase());
}

function hasWeakGeneratedDetail(value) {
	const text = String(value ?? "");
	return hasPlaceholder(text) || text.includes("?") || /\bunresolved item\b/i.test(text);
}

function validateCoverage(backlog, readiness, completion, liveEvidence, errors) {
	const tasks = new Map((backlog.tasks ?? []).map((task) => [task.id, task]));
	for (const blocker of readiness?.blockers ?? []) {
		validateCoveredTask(tasks, `resolve-${blocker.area}`, blocker.blocking_items, errors);
	}
	for (const item of completion?.requirements ?? []) {
		if (item.status === "complete" || item.status === "control-active") continue;
		validateCoveredTask(tasks, `complete-${item.id}`, 1, errors);
	}
	for (const requirement of liveEvidence?.requirements ?? []) {
		if (requirement.ready) continue;
		const missing = requirement.missing_evidence ?? [];
		validateCoveredTask(tasks, `attach-live-evidence-${requirement.id}`, Math.max(1, missing.length), errors);
	}
	const missingEvidenceItems = (liveEvidence?.requirements ?? []).reduce(
		(sum, requirement) => sum + (requirement.missing_evidence?.length ?? 0),
		0,
	);
	if (backlog.status.live_evidence_missing_items !== missingEvidenceItems) {
		errors.push(`backlog live_evidence_missing_items must match live evidence missing item total ${missingEvidenceItems}`);
	}
}

function validateCoveredTask(tasks, id, blockingItems, errors) {
	const task = tasks.get(id);
	if (!task) {
		errors.push(`${id}: missing generated backlog task`);
		return;
	}
	if (task.status !== "blocked") errors.push(`${id}: uncovered blocker task must be blocked`);
	if (task.blocking_items !== blockingItems) {
		errors.push(`${id}: blocking_items must match source blocker count`);
	}
}
