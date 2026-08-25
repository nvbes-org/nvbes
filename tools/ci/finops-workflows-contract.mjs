import assert from "node:assert/strict";
import { parse } from "yaml";

const FINOPS_COMMAND = "pnpm check:finops";
const CHECKOUT_ACTION =
	"actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1";
const SETUP_NODE_ACTION =
	"actions/setup-node@820762786026740c76f36085b0efc47a31fe5020";
const COREPACK_COMMAND =
	"corepack enable\ncorepack prepare pnpm@11.18.0 --activate";
const LOCKED_INSTALL_COMMAND =
	"pnpm install --frozen-lockfile --prefer-offline";

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

function isExactUsesStep(step, action) {
	return isRecord(step) && step.uses === action && !Object.hasOwn(step, "run");
}

function isExactRunStep(step, command) {
	return isRecord(step) && step.run === command && !Object.hasOwn(step, "uses");
}

function assertPreGatePrefix(gate, contract) {
	const prefix = gate.steps.slice(0, 6);
	assert.equal(
		prefix.length,
		6,
		`${contract.job} must use the allowlisted pre-gate step prefix`,
	);
	const [checkout, setupNode, security, corepack, install, finOps] = prefix;
	assert.ok(
		isExactUsesStep(checkout, CHECKOUT_ACTION),
		`${contract.job} must use the allowlisted pre-gate step prefix`,
	);
	assert.ok(
		isExactUsesStep(setupNode, SETUP_NODE_ACTION) &&
			String(setupNode.with?.["node-version"]) === "24",
		`${contract.job} must use the allowlisted pre-gate step prefix`,
	);
	assert.ok(
		isExactRunStep(
			security,
			`node tools/security/check-ci-cd-security.mjs --workflow ${contract.workflowPath}`,
		),
		`${contract.job} must use the allowlisted pre-gate step prefix`,
	);
	assert.ok(
		isExactRunStep(corepack, COREPACK_COMMAND),
		`${contract.job} must use the allowlisted pre-gate step prefix`,
	);
	assert.ok(
		isExactRunStep(install, LOCKED_INSTALL_COMMAND),
		`${contract.job} must use the allowlisted pre-gate step prefix`,
	);
	assert.ok(
		isExactRunStep(finOps, FINOPS_COMMAND),
		`${contract.job} must use the allowlisted pre-gate step prefix`,
	);
	assertCriticalSteps(
		prefix.map((step) => ({ step })),
		contract.job,
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
	assert.equal(
		typeof contract.workflowPath,
		"string",
		"workflow contract must define its workflow path",
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
	assertPreGatePrefix(gate, contract);

	const terraformSteps = gateRunSteps.filter(
		({ run, stepIndex }) =>
			run === contract.terraformValidation && stepIndex > 5,
	);
	assert.equal(
		terraformSteps.length,
		1,
		"Terraform validation must run after pnpm check:finops",
	);
	assertCriticalSteps(terraformSteps, contract.job);

	assertDownstreamJobs(workflow, contract);
}
