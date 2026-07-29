import { isDeepStrictEqual } from "node:util";
import { parse } from "yaml";
import { validateWorkflowJobs } from "./workflow-job-contract.mjs";

const inputContract = {
	duration: {
		default: "profile-default",
		options: ["profile-default", "6m", "15m", "30m", "4h"],
		required: true,
		type: "choice",
	},
	lane: {
		default: "nightly",
		options: ["nightly", "resilience"],
		required: true,
		type: "choice",
	},
	profile: {
		default: "stress",
		options: ["volume", "spike", "stress", "soak"],
		required: true,
		type: "choice",
	},
	service_url: { required: false, type: "string" },
	web_url: { required: false, type: "string" },
};

export function validateAccountQualityWorkflow(workflowSource, automation) {
	const workflow = parseWorkflow(workflowSource);
	validateTriggers(workflow.on, automation);
	validateGlobalControls(workflow);
	validateWorkflowJobs(workflow.jobs);
	rejectSecretInputs(workflow);
}

export function parseWorkflow(workflowSource) {
	let workflow;
	try {
		workflow = parse(workflowSource);
	} catch (error) {
		throw new Error(
			`account-quality workflow is invalid YAML: ${error.message}`,
			{ cause: error },
		);
	}
	assert(isRecord(workflow), "account-quality workflow must be an object");
	return workflow;
}

function validateTriggers(triggers, automation) {
	assert(isRecord(triggers), "workflow triggers are missing");
	assertExactKeys(
		triggers,
		["schedule", "workflow_dispatch"],
		"workflow triggers",
	);
	assert(
		Array.isArray(triggers.schedule),
		"workflow schedule must be an array",
	);
	assertDeepEqual(
		triggers.schedule.map((entry) => entry?.cron),
		[automation.scheduledDailyCron, automation.scheduledWeeklyCron],
		"workflow crons",
	);

	const dispatch = triggers.workflow_dispatch;
	assert(isRecord(dispatch), "workflow_dispatch must be an object");
	assertExactKeys(dispatch, ["inputs"], "workflow_dispatch");
	assert(isRecord(dispatch.inputs), "workflow_dispatch inputs are missing");
	assertExactKeys(
		dispatch.inputs,
		Object.keys(inputContract),
		"workflow_dispatch inputs",
	);

	for (const [name, expected] of Object.entries(inputContract)) {
		validateInput(name, dispatch.inputs[name], expected);
	}
}

function validateInput(name, actual, expected) {
	assert(isRecord(actual), `workflow input ${name} must be an object`);
	assert(
		nonEmpty(actual.description),
		`workflow input ${name} requires a description`,
	);
	const actualContract = Object.fromEntries(
		Object.keys(expected).map((key) => [key, actual[key]]),
	);
	assertDeepEqual(actualContract, expected, `workflow input ${name}`);
	assertExactKeys(
		actual,
		["description", ...Object.keys(expected)],
		`workflow input ${name}`,
	);
}

function validateGlobalControls(workflow) {
	assertDeepEqual(
		workflow.concurrency,
		{ "cancel-in-progress": false, group: "account-quality-campaign" },
		"workflow concurrency",
	);
	assertDeepEqual(
		workflow.permissions,
		{ actions: "read", contents: "read" },
		"workflow permissions",
	);
	assert(
		workflow.env?.CI === "true",
		"workflow must define CI as the string true",
	);
	assert(
		workflow.defaults?.run?.shell ===
			"bash --noprofile --norc -euo pipefail {0}",
		"workflow must use the fail-closed bash shell",
	);
}

function rejectSecretInputs(workflow) {
	assert(
		!JSON.stringify(workflow).includes("ACCOUNT_TEST_COOKIE"),
		"public GitHub workflows must never accept an authenticated session cookie",
	);
}

function assertExactKeys(record, expected, label) {
	assertDeepEqual(
		Object.keys(record).sort(),
		[...expected].sort(),
		`${label} keys`,
	);
}

function assertDeepEqual(actual, expected, label) {
	assert(
		isDeepStrictEqual(actual, expected),
		`${label} differs from its exact contract`,
	);
}

function nonEmpty(value) {
	return typeof value === "string" && value.trim().length > 0;
}

function assert(condition, message) {
	if (!condition) {
		throw new Error(message);
	}
}

function isRecord(value) {
	return typeof value === "object" && value !== null && !Array.isArray(value);
}
