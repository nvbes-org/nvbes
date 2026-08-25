import assert from "node:assert/strict";
import { parse } from "yaml";

function isRecord(value) {
	return value !== null && typeof value === "object" && !Array.isArray(value);
}

function commandEntriesForJob(job) {
	assert.ok(isRecord(job), "workflow job must be an object");
	assert.ok(Array.isArray(job.steps), "workflow job must contain steps");

	return job.steps.flatMap((step, stepIndex) => {
		if (!isRecord(step) || typeof step.run !== "string") return [];

		return step.run
			.split(/\r?\n/u)
			.map((line, lineIndex) => ({
				command: line.trim(),
				lineIndex,
				step,
				stepIndex,
			}))
			.filter(({ command }) => command.length > 0 && !command.startsWith("#"));
	});
}

function isFinOpsCommand(command) {
	return command === "pnpm check:finops";
}

function isLockedInstall(command) {
	return (
		command === "pnpm install --frozen-lockfile" ||
		command === "pnpm install --frozen-lockfile --prefer-offline"
	);
}

function containsTerraformToken(command) {
	return /(^|[^A-Za-z0-9_])terraform(?=$|[^A-Za-z0-9_])/u.test(command);
}

function isDirectTerraformCommand(command) {
	return /^terraform(?:\s|$)/u.test(command);
}

function isFailClosedStep(step) {
	return (
		!Object.hasOwn(step, "if") &&
		(!Object.hasOwn(step, "continue-on-error") ||
			step["continue-on-error"] === false)
	);
}

function assertFailClosedGateJob(gate, jobName) {
	assert.ok(isRecord(gate), "workflow job must be an object");
	assert.ok(
		!Object.hasOwn(gate, "if") &&
			(!Object.hasOwn(gate, "continue-on-error") ||
				gate["continue-on-error"] === false),
		`${jobName} must be unconditional and fail closed`,
	);
}

function assertFailClosedCriticalSteps(entries, jobName) {
	const criticalSteps = new Set(entries.map(({ step }) => step));
	assert.equal(
		[...criticalSteps].every(isFailClosedStep),
		true,
		`critical ${jobName} steps must be unconditional and fail closed`,
	);
}

function jobNeeds(job) {
	if (!isRecord(job)) return [];
	if (typeof job.needs === "string") return [job.needs];
	return Array.isArray(job.needs) ? job.needs : [];
}

export function validateWorkflowContract(source, contract) {
	const workflow = parse(source);
	assert.ok(
		isRecord(workflow) && isRecord(workflow.jobs),
		"workflow must define jobs",
	);

	const gate = workflow.jobs[contract.job];
	assertFailClosedGateJob(gate, contract.job);
	const gateCommands = commandEntriesForJob(gate);
	const gateFinOpsIndexes = gateCommands
		.map(({ command }, index) => (isFinOpsCommand(command) ? index : -1))
		.filter((index) => index >= 0);

	assert.equal(
		gateFinOpsIndexes.length,
		1,
		`${contract.job} must invoke pnpm check:finops exactly once`,
	);

	for (const [jobName, job] of Object.entries(workflow.jobs)) {
		if (jobName === contract.job) continue;
		assert.equal(
			commandEntriesForJob(job).filter(({ command }) =>
				isFinOpsCommand(command),
			).length,
			0,
			`${jobName} must not invoke pnpm check:finops`,
		);
	}

	const finOpsIndex = gateFinOpsIndexes[0];
	const commandsBeforeGate = gateCommands.slice(0, finOpsIndex);
	const commandsAfterGate = gateCommands.slice(finOpsIndex + 1);
	const lockedInstalls = commandsBeforeGate.filter(({ command }) =>
		isLockedInstall(command),
	);
	const terraformValidations = commandsAfterGate.filter(({ command }) =>
		isDirectTerraformCommand(command),
	);

	assert.ok(
		lockedInstalls.length > 0,
		"locked pnpm install must run before pnpm check:finops",
	);
	assert.equal(
		commandsBeforeGate.some(({ command }) => containsTerraformToken(command)),
		false,
		"Terraform command must not run before pnpm check:finops",
	);
	assert.ok(
		terraformValidations.length > 0,
		"Terraform validation must run after pnpm check:finops",
	);
	assertFailClosedCriticalSteps(
		[...lockedInstalls, gateCommands[finOpsIndex], ...terraformValidations],
		contract.job,
	);

	if (contract.buildJob === undefined || contract.deployJob === undefined)
		return;
	assert.ok(
		jobNeeds(workflow.jobs[contract.buildJob]).includes(contract.job),
		`${contract.buildJob} must need ${contract.job}`,
	);
	assert.equal(
		workflow.jobs[contract.buildJob].if,
		contract.buildIf,
		`${contract.buildJob} must use the required success condition`,
	);
	const deployNeeds = jobNeeds(workflow.jobs[contract.deployJob]);
	assert.ok(
		deployNeeds.includes(contract.job),
		`${contract.deployJob} must need ${contract.job}`,
	);
	assert.ok(
		deployNeeds.includes(contract.buildJob),
		`${contract.deployJob} must need ${contract.buildJob}`,
	);
	assert.equal(
		workflow.jobs[contract.deployJob].if,
		contract.deployIf,
		`${contract.deployJob} must use the required success and branch condition`,
	);
}
