import assert from "node:assert/strict";
import { parse } from "yaml";

const FINOPS_COMMAND = "pnpm check:finops";
const CHECKOUT_ACTION =
	"actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1";
const SETUP_NODE_ACTION =
	"actions/setup-node@820762786026740c76f36085b0efc47a31fe5020";
const SETUP_TERRAFORM_ACTION =
	"hashicorp/setup-terraform@dfe3c3f87815947d99a8997f908cb6525fc44e9e";
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
			({ step }) =>
				isRecord(step) && isFailClosed(step) && !Object.hasOwn(step, "shell"),
		),
		true,
		`critical ${jobName} steps must be unconditional and fail closed`,
	);
}

function terraformInitCommand(contract) {
	return `terraform -chdir=${contract.terraformEnvironmentPath} init -backend=false -input=false`;
}

function terraformValidationCommand(contract) {
	return `terraform -chdir=${contract.terraformEnvironmentPath} validate`;
}

function expectedGatePrefix(contract) {
	return [
		{
			uses: CHECKOUT_ACTION,
			with: { "fetch-depth": 0, "persist-credentials": false },
		},
		{ uses: SETUP_NODE_ACTION, with: { "node-version": 24 } },
		{
			name: "CI/CD security gate",
			run: `node tools/security/check-ci-cd-security.mjs --workflow ${contract.workflowPath}`,
		},
		{ name: "Enable pnpm", run: COREPACK_COMMAND },
		{ name: "Install locked dependencies", run: LOCKED_INSTALL_COMMAND },
		{ name: "Enforce FinOps contract", run: FINOPS_COMMAND },
		{
			uses: SETUP_TERRAFORM_ACTION,
			with: { terraform_version: "1.15.8", terraform_wrapper: false },
		},
		{
			name: contract.terraformInitStepName,
			run: terraformInitCommand(contract),
		},
		{
			name: contract.terraformValidationStepName,
			run: terraformValidationCommand(contract),
		},
	];
}

function assertGatePrefix(gate, contract) {
	const prefix = gate.steps.slice(0, 9);
	assert.equal(
		prefix.length,
		9,
		`${contract.job} must use the allowlisted gate step prefix`,
	);
	assertCriticalSteps(
		prefix.map((step) => ({ step })),
		contract.job,
	);
	assert.deepEqual(
		prefix,
		expectedGatePrefix(contract),
		`${contract.job} must use the allowlisted gate step prefix`,
	);
}

function assertExecutionContext(workflow, gate, contract) {
	assert.deepEqual(
		workflow.defaults,
		{ run: { shell: contract.shell } },
		"workflow defaults must exactly define the fail-closed shell",
	);
	assert.equal(
		Object.hasOwn(workflow, "env"),
		false,
		"workflow env is forbidden",
	);
	assert.deepEqual(
		gate.env,
		contract.gateEnv,
		`${contract.job} env must match`,
	);
	assert.deepEqual(
		gate["runs-on"],
		contract.runsOn,
		`${contract.job} runner labels must match`,
	);
	assert.equal(
		Object.hasOwn(gate, "defaults") || Object.hasOwn(gate, "container"),
		false,
		`${contract.job} defaults and container are forbidden`,
	);
}

function jobNeeds(job) {
	if (!isRecord(job)) return [];
	if (typeof job.needs === "string") return [job.needs];
	return Array.isArray(job.needs) ? job.needs : [];
}

function assertJobInventory(workflow, contract) {
	const expectedJobs = [contract.job, contract.buildJob, contract.deployJob]
		.filter((jobName) => jobName !== undefined)
		.sort();
	const actualJobs = Object.keys(workflow.jobs).sort();
	const matches =
		actualJobs.length === expectedJobs.length &&
		actualJobs.every((jobName, index) => jobName === expectedJobs[index]);
	if (!matches) {
		assert.fail(
			`workflow jobs must exactly match contract: ${expectedJobs.join(", ")}`,
		);
	}
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
		typeof contract.workflowPath,
		"string",
		"workflow contract must define its workflow path",
	);
	assert.equal(
		typeof contract.terraformEnvironmentPath,
		"string",
		"workflow contract must define its Terraform environment path",
	);
	assert.equal(
		typeof contract.terraformInitStepName,
		"string",
		"workflow contract must define its Terraform init step name",
	);
	assert.equal(
		typeof contract.terraformValidationStepName,
		"string",
		"workflow contract must define its Terraform validation step name",
	);

	const gate = workflow.jobs[contract.job];
	assertFailClosedGateJob(gate, contract.job, contract.shell);
	assertExecutionContext(workflow, gate, contract);
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
	assertGatePrefix(gate, contract);

	const terraformSteps = gateRunSteps.filter(
		({ run, stepIndex }) =>
			run === terraformValidationCommand(contract) && stepIndex > 5,
	);
	assert.equal(
		terraformSteps.length,
		1,
		"Terraform validation must run after pnpm check:finops",
	);
	assertCriticalSteps(terraformSteps, contract.job);

	assertDownstreamJobs(workflow, contract);
	assertJobInventory(workflow, contract);
}
