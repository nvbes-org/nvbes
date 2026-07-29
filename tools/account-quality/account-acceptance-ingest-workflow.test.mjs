import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { parse, stringify } from "yaml";

const workflowSource = await readFile(
	".github/workflows/account-acceptance-ingest.yml",
	"utf8",
);
const registry = JSON.parse(
	await readFile("docs/security/ci-cd-security-controls.json", "utf8"),
);
const expression = (value) => `\${{ ${value} }}`;

test("ingest verifies an exact-SHA signed source before immutable publication", () => {
	assert.doesNotThrow(() => validateIngestWorkflow(workflowSource));
});

test("ingest rejects dispatch inputs and Environment drift", () => {
	assert.throws(
		() =>
			validateIngestWorkflow(
				mutateWorkflow((workflow) => {
					workflow.on.workflow_dispatch = {
						inputs: { source_run_id: { type: "string" } },
					};
				}),
			),
		/ingest dispatch must not accept inputs/u,
	);
	assert.throws(
		() =>
			validateIngestWorkflow(
				mutateWorkflow((workflow) => {
					workflow.jobs["verify-and-publish"].environment.name = "production";
				}),
			),
		/ingest job must target account-acceptance/u,
	);
});

test("ingest rejects mutable source and final artifact selectors", () => {
	assert.throws(
		() =>
			validateIngestWorkflow(
				mutateWorkflow((workflow) => {
					namedStep(workflow, "Download exact-SHA signed packet").with.name =
						"account-acceptance-source-latest";
				}),
			),
		/source artifact must bind the exact github SHA/u,
	);
	assert.throws(
		() =>
			validateIngestWorkflow(
				mutateWorkflow((workflow) => {
					namedStep(
						workflow,
						"Publish immutable exact-SHA acceptance evidence",
					).with.name = "account-acceptance-latest";
				}),
			),
		/published artifact must bind the exact github SHA/u,
	);
});

test("ingest rejects publication without cryptographic verification", () => {
	assert.throws(
		() =>
			validateIngestWorkflow(
				mutateWorkflow((workflow) => {
					namedStep(workflow, "Verify signed Account acceptance packet").run =
						"echo trusted";
				}),
			),
		/must run the acceptance evidence verifier/u,
	);
});

test("ingest rejects raw source publication or sanitizer reordering", () => {
	assert.throws(
		() =>
			validateIngestWorkflow(
				mutateWorkflow((workflow) => {
					namedStep(
						workflow,
						"Publish immutable exact-SHA acceptance evidence",
					).with.path =
						`${expression("runner.temp")}/account-acceptance-source/`;
				}),
			),
		/publish only the fresh exact publication/u,
	);
	assert.throws(
		() =>
			validateIngestWorkflow(
				mutateWorkflow((workflow) => {
					const steps = workflow.jobs["verify-and-publish"].steps;
					const sanitizer = steps.splice(
						steps.indexOf(
							namedStep(workflow, "Build exact verified publication"),
						),
						1,
					)[0];
					steps.splice(
						steps.indexOf(
							namedStep(workflow, "Verify signed Account acceptance packet"),
						),
						0,
						sanitizer,
					);
				}),
			),
		/verification must precede exact publication/u,
	);
});

function validateIngestWorkflow(source) {
	const workflow = parse(source);
	assert.deepEqual(Object.keys(workflow.on ?? {}), ["workflow_dispatch"]);
	requireCondition(
		workflow.on.workflow_dispatch == null ||
			Object.keys(workflow.on.workflow_dispatch).length === 0,
		"ingest dispatch must not accept inputs",
	);
	requireCondition(
		!source.includes("inputs."),
		"ingest cannot trust workflow inputs",
	);
	assert.deepEqual(workflow.permissions, { actions: "read", contents: "read" });
	assert.deepEqual(workflow.concurrency, {
		"cancel-in-progress": false,
		group: `account-acceptance-ingest-${expression("github.sha")}`,
	});
	assert.equal(
		workflow.defaults?.run?.shell,
		"bash --noprofile --norc -euo pipefail {0}",
	);
	assert.deepEqual(Object.keys(workflow.jobs ?? {}), ["verify-and-publish"]);

	const job = workflow.jobs["verify-and-publish"];
	assert.equal(
		job.environment?.name,
		"account-acceptance",
		"ingest job must target account-acceptance",
	);
	assert.equal(job["runs-on"], "ubuntu-latest");
	assert.equal(job["timeout-minutes"], 30);
	assert.equal(
		job.env.NVBES_RELEASE_SHA,
		expression("github.sha"),
		"ingest release must bind github.sha",
	);
	assert.equal(job.env.NVBES_RELEASE_REF, expression("github.ref"));
	assert.equal(job.env.NVBES_RELEASE_REF_NAME, expression("github.ref_name"));
	assert.equal(job.env.NVBES_RELEASE_REF_TYPE, expression("github.ref_type"));
	assert.equal(
		job.env.ACCOUNT_INGEST_PUBLICATION_ROOT,
		`${expression("runner.temp")}/account-acceptance-publication`,
	);
	for (const variable of [
		"ACCOUNT_ACCEPTANCE_TRUSTED_KEY_ID",
		"ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_SHA256",
		"ACCOUNT_DEPLOYMENT_EXPECTED_COMPONENT_UIDS",
		"ACCOUNT_DEPLOYMENT_TRUSTED_KEY_ID",
		"ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_SHA256",
	]) {
		assert.equal(
			job.env[variable],
			expression(`vars.${variable}`),
			`${variable} must come from protected vars`,
		);
	}

	const checkout = actionStep(job, "actions/checkout");
	assert.equal(
		checkout.uses,
		"actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1",
	);
	assert.deepEqual(checkout.with, {
		"fetch-depth": 1,
		"persist-credentials": false,
		ref: expression("github.ref"),
	});
	assert.equal(
		namedStep(job, "Validate immutable Account RC tag").run,
		"node tools/account-quality/verify-account-release-ref.mjs",
	);
	assert.deepEqual(actionStep(job, "actions/setup-node").with, {
		"node-version": 24,
	});
	assert.equal(source.includes("pnpm install"), false);

	const sourceProof = namedStep(
		job,
		"Validate signed-packet source provenance",
	);
	assert.equal(
		sourceProof.env.ACCEPTANCE_SOURCE_RUN_ID,
		expression("vars.ACCOUNT_ACCEPTANCE_SOURCE_RUN_ID"),
	);
	assert.equal(
		sourceProof.env.EXPECTED_RELEASE_TAG,
		expression("github.ref_name"),
	);
	assert.equal(sourceProof.env.ACCEPTANCE_SOURCE_WORKFLOW_PATH, undefined);
	for (const proof of [
		'run.path !== ".github/workflows/account-acceptance-source.yml"',
		"run.head_branch !== expectedTag",
		"run.head_sha !== release",
		'run.conclusion !== "success"',
		"run.repository?.full_name !== repository",
		'run.event !== "workflow_dispatch"',
	]) {
		assert.ok(sourceProof.run.includes(proof), `missing source proof ${proof}`);
	}

	const download = namedStep(job, "Download exact-SHA signed packet");
	assert.equal(
		download.uses,
		"actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c",
	);
	assert.equal(
		download.with.name,
		`account-acceptance-source-${expression("github.sha")}`,
		"source artifact must bind the exact github SHA",
	);
	assert.equal(
		download.with["run-id"],
		expression("vars.ACCOUNT_ACCEPTANCE_SOURCE_RUN_ID"),
	);
	assert.equal(download.with["github-token"], expression("github.token"));
	assert.equal(download.with.repository, expression("github.repository"));

	const trust = namedStep(job, "Materialize acceptance trust roots");
	assert.deepEqual(trust.env, {
		ACCEPTANCE_TRUSTED_PUBLIC_KEY_PEM: expression(
			"secrets.ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_PEM",
		),
		DEPLOYMENT_TRUSTED_PUBLIC_KEY_PEM: expression(
			"secrets.ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_PEM",
		),
	});
	assert.equal(
		namedStep(job, "Verify signed Account acceptance packet").run,
		"node tools/account-quality/verify-acceptance-evidence.mjs",
		"ingest must run the acceptance evidence verifier",
	);
	const verificationIndex = job.steps.indexOf(
		namedStep(job, "Verify signed Account acceptance packet"),
	);
	const sanitizer = namedStep(job, "Build exact verified publication");
	assert.match(
		sanitizer.run,
		/prepare-account-acceptance-publication\.mjs[\s\S]+\$ACCOUNT_INGEST_EVIDENCE_ROOT[\s\S]+\$ACCOUNT_INGEST_PUBLICATION_ROOT/u,
	);
	const sanitizerIndex = job.steps.indexOf(sanitizer);
	const upload = namedStep(
		job,
		"Publish immutable exact-SHA acceptance evidence",
	);
	const uploadIndex = job.steps.indexOf(upload);
	requireCondition(
		verificationIndex >= 0 &&
			verificationIndex < sanitizerIndex &&
			sanitizerIndex < uploadIndex,
		"verification must precede exact publication and upload",
	);
	assert.equal(
		upload.with.name,
		`account-acceptance-${expression("github.sha")}`,
		"published artifact must bind the exact github SHA",
	);
	assert.equal(upload.with["if-no-files-found"], "error");
	assert.equal(upload.with["retention-days"], 90);
	assert.equal(upload.if, undefined);
	requireCondition(
		upload.with.path ===
			`${expression("runner.temp")}/account-acceptance-publication/`,
		"ingest must publish only the fresh exact publication",
	);

	for (const step of job.steps) {
		if (step.uses) {
			assert.match(
				step.uses,
				/@[a-f0-9]{40}$/u,
				`${step.uses} is not SHA-pinned`,
			);
		}
	}
	for (const secret of extractExpressions(source, "secrets")) {
		assert.ok(
			registry.allowedSecrets.includes(secret),
			`${secret} is not allowlisted`,
		);
	}
	assert.ok(registry.allowedActions.includes(download.uses));
	const securityGateIndex = job.steps.indexOf(
		namedStep(job, "CI/CD security gate"),
	);
	const trustIndex = job.steps.indexOf(
		namedStep(job, "Materialize acceptance trust roots"),
	);
	assert.ok(securityGateIndex >= 0 && securityGateIndex < trustIndex);
}

function mutateWorkflow(mutation) {
	const workflow = parse(workflowSource);
	mutation(workflow);
	return stringify(workflow);
}

function namedStep(jobOrWorkflow, name) {
	const job = jobOrWorkflow.steps
		? jobOrWorkflow
		: jobOrWorkflow.jobs["verify-and-publish"];
	const matches = job.steps.filter((step) => step.name === name);
	assert.equal(matches.length, 1, `expected exactly one ${name} step`);
	return matches[0];
}

function actionStep(job, prefix) {
	const matches = job.steps.filter((step) =>
		step.uses?.startsWith(`${prefix}@`),
	);
	assert.equal(matches.length, 1, `expected exactly one ${prefix} step`);
	return matches[0];
}

function extractExpressions(source, context) {
	return [
		...source.matchAll(new RegExp(`${context}\\.([A-Z0-9_]+)`, "gu")),
	].map((match) => match[1]);
}

function requireCondition(condition, message) {
	if (!condition) {
		throw new Error(message);
	}
}
