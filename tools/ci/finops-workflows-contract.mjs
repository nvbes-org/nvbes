import assert from "node:assert/strict";
import { parse } from "yaml";

const FINOPS_COMMAND = "pnpm check:finops";
const LOCKED_INSTALL_COMMANDS = new Set([
	"pnpm install --frozen-lockfile",
	"pnpm install --frozen-lockfile --prefer-offline",
]);

function isRecord(value) {
	return value !== null && typeof value === "object" && !Array.isArray(value);
}

function runSteps(job) {
	assert.ok(isRecord(job), "workflow job must be an object");
	assert.ok(Array.isArray(job.steps), "workflow job must contain steps");

	return job.steps.flatMap((step, stepIndex) => {
		if (!isRecord(step) || typeof step.run !== "string") return [];
		return [{ run: step.run, step, stepIndex }];
	});
}

function containsTerraformToken(command) {
	return /(^|[^A-Za-z0-9_])terraform(?=$|[^A-Za-z0-9_])/u.test(command);
}

function isFailClosed(value) {
	return (
		!Object.hasOwn(value, "if") &&
		(!Object.hasOwn(value, "continue-on-error") ||
			value["continue-on-error"] === false)
	);
}

function assertFailClosedGateJob(gate, jobName, shell) {
	assert.ok(isRecord(gate), "workflow job must be an object");
	assert.equal(
		isFailClosed(gate),
		true,
		`${jobName} must be unconditional and fail closed`,
	);

	const jobShell = gate.defaults?.run?.shell;
	assert.ok(
		jobShell === undefined || jobShell === shell,
		`${jobName} must not override the fail-closed shell`,
	);
}

function assertCriticalSteps(entries, jobName) {
	assert.equal(
		entries.every(
			({ step }) => isFailClosed(step) && !Object.hasOwn(step, "shell"),
		),
		true,
		`critical ${jobName} steps must be unconditional and fail closed`,
	);
}

function jobNeeds(job) {
	if (!isRecord(job)) return [];
	if (typeof job.needs === "string") return [job.needs];
	return Array.isArray(job.needs) ? job.needs : [];
}

function assertDownstreamJobs(workflow, contract) {
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

export function validateWorkflowContract(source, contract) {
	const workflow = parse(source);
	assert.ok(
		isRecord(workflow) && isRecord(workflow.jobs),
		"workflow must define jobs",
	);
	assert.equal(
		workflow.defaults?.run?.shell,
		contract.shell,
		"workflow must use the required fail-closed default shell",
	);
	assert.equal(
		typeof contract.terraformValidation,
		"string",
		"workflow contract must define its Terraform validation",
	);

	const gate = workflow.jobs[contract.job];
	assertFailClosedGateJob(gate, contract.job, contract.shell);
	const gateRunSteps = runSteps(gate);
	const finOpsSteps = gateRunSteps.filter(({ run }) => run === FINOPS_COMMAND);
	assert.equal(
		finOpsSteps.length,
		1,
		`${contract.job} must invoke pnpm check:finops exactly once`,
	);

	for (const [jobName, job] of Object.entries(workflow.jobs)) {
		if (jobName === contract.job) continue;
		assert.equal(
			runSteps(job).filter(({ run }) => run === FINOPS_COMMAND).length,
			0,
			`${jobName} must not invoke pnpm check:finops`,
		);
	}
	assert.equal(
		gateRunSteps.some(
			({ run }) => run !== FINOPS_COMMAND && run.includes(FINOPS_COMMAND),
		),
		false,
		`${contract.job} FinOps gate must be a dedicated exact step`,
	);

	const finOpsStep = finOpsSteps[0];
	const commandsBeforeGate = runSteps({
		steps: gate.steps.slice(0, finOpsStep.stepIndex),
	});
	assert.equal(
		commandsBeforeGate.some(({ run }) => containsTerraformToken(run)),
		false,
		"Terraform command must not run before pnpm check:finops",
	);

	const precedingStep = gate.steps[finOpsStep.stepIndex - 1];
	const installStep =
		isRecord(precedingStep) &&
		typeof precedingStep.run === "string" &&
		LOCKED_INSTALL_COMMANDS.has(precedingStep.run)
			? { run: precedingStep.run, step: precedingStep }
			: undefined;
	assert.ok(
		installStep,
		"locked pnpm install must run before pnpm check:finops",
	);

	const terraformSteps = gateRunSteps.filter(
		({ run, stepIndex }) =>
			run === contract.terraformValidation && stepIndex > finOpsStep.stepIndex,
	);
	assert.equal(
		terraformSteps.length,
		1,
		"Terraform validation must run after pnpm check:finops",
	);
	assertCriticalSteps(
		[installStep, finOpsStep, terraformSteps[0]],
		contract.job,
	);

	assertDownstreamJobs(workflow, contract);
}
