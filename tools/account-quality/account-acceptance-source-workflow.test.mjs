import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { parse, stringify } from "yaml";

const workflowSource = await readFile(
	".github/workflows/account-acceptance-source.yml",
	"utf8",
);
const manifest = JSON.parse(
	await readFile("docs/testing/account-test-manifest.json", "utf8"),
);
const expression = (value) => `\${{ ${value} }}`;

test("source workflow fails closed without evidence credentials or publication", () => {
	assert.doesNotThrow(() => validateBlockedSourceWorkflow(workflowSource));
});

test("source workflow rejects dispatch inputs and Environment drift", () => {
	assert.throws(
		() =>
			validateBlockedSourceWorkflow(
				mutateWorkflow((workflow) => {
					workflow.on.workflow_dispatch = {
						inputs: { asset: { type: "string" } },
					};
				}),
			),
		/source dispatch must not accept inputs/u,
	);
	assert.throws(
		() =>
			validateBlockedSourceWorkflow(
				mutateWorkflow((workflow) => {
					workflow.jobs["block-untrusted-import"].environment.name =
						"account-acceptance";
				}),
			),
		/must target account-acceptance-source/u,
	);
});

test("source workflow rejects removal or weakening of the fail-closed step", () => {
	for (const replacement of [
		"echo warning",
		"node tools/account-quality/prepare-account-acceptance-publication.mjs",
	]) {
		assert.throws(
			() =>
				validateBlockedSourceWorkflow(
					mutateWorkflow((workflow) => {
						namedStep(
							workflow.jobs["block-untrusted-import"],
							"Block candidate-controlled evidence import",
						).run = replacement;
					}),
				),
			/blocker must be the unconditional terminal fatal step/u,
		);
	}
});

test("source workflow keeps the unconditional blocker terminal and fatal", () => {
	for (const mutate of [
		(workflow) => {
			workflow.jobs["block-untrusted-import"].steps.push({
				name: "Continue after blocker",
				run: "echo unsafe",
			});
		},
		(workflow) => {
			workflow.jobs["block-untrusted-import"].if = "always()";
		},
		(workflow) => {
			workflow.jobs["block-untrusted-import"]["continue-on-error"] = true;
		},
		(workflow) => {
			namedStep(
				workflow.jobs["block-untrusted-import"],
				"Block candidate-controlled evidence import",
			).if = "always()";
		},
		(workflow) => {
			namedStep(
				workflow.jobs["block-untrusted-import"],
				"Block candidate-controlled evidence import",
			)["continue-on-error"] = true;
		},
	]) {
		assert.throws(
			() => validateBlockedSourceWorkflow(mutateWorkflow(mutate)),
			/blocker must be the unconditional terminal fatal step/u,
		);
	}
});

test("manifest keeps confidential evidence import release-blocking", () => {
	const gap = manifest.knownGaps.find(({ description }) =>
		description.includes("externally pinned evidence importer"),
	);
	assert.ok(gap, "missing externally pinned evidence importer gap");
	assert.equal(gap.releaseBlocking, true);
	assert.match(gap.description, /DNS connection binding/u);
	assert.match(gap.description, /TOCTOU/u);
	for (const category of [
		"compliance",
		"penetration",
		"security",
		"user-acceptance",
	]) {
		assert.ok(gap.categories.includes(category));
		assert.ok(manifest.releaseReadiness.blockingCategories.includes(category));
	}
});

function validateBlockedSourceWorkflow(source) {
	const workflow = parse(source);
	assert.deepEqual(Object.keys(workflow.on ?? {}), ["workflow_dispatch"]);
	requireCondition(
		workflow.on.workflow_dispatch == null ||
			Object.keys(workflow.on.workflow_dispatch).length === 0,
		"source dispatch must not accept inputs",
	);
	requireCondition(
		!source.includes("inputs."),
		"source cannot trust dispatch inputs",
	);
	assert.deepEqual(workflow.permissions, { actions: "read", contents: "read" });
	assert.deepEqual(workflow.concurrency, {
		"cancel-in-progress": false,
		group: `account-acceptance-source-${expression("github.sha")}`,
	});
	assert.equal(
		workflow.defaults?.run?.shell,
		"bash --noprofile --norc -euo pipefail {0}",
	);
	assert.deepEqual(Object.keys(workflow.jobs ?? {}), [
		"block-untrusted-import",
	]);

	const job = workflow.jobs["block-untrusted-import"];
	requireCondition(
		job.environment?.name === "account-acceptance-source",
		"source job must target account-acceptance-source",
	);
	assert.equal(job["runs-on"], "ubuntu-latest");
	assert.equal(job["timeout-minutes"], 10);
	requireCondition(
		job.if === undefined &&
			job["continue-on-error"] === undefined &&
			job.steps.every((step) => step["continue-on-error"] === undefined),
		"source blocker must be the unconditional terminal fatal step",
	);
	assert.equal(job.env.NVBES_RELEASE_SHA, expression("github.sha"));
	assert.equal(job.env.NVBES_RELEASE_REF, expression("github.ref"));
	assert.equal(job.env.NVBES_RELEASE_REF_NAME, expression("github.ref_name"));
	assert.equal(job.env.NVBES_RELEASE_REF_TYPE, expression("github.ref_type"));

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
	const blocker = namedStep(job, "Block candidate-controlled evidence import");
	requireCondition(
		job.steps.at(-1) === blocker &&
			blocker.if === undefined &&
			blocker["continue-on-error"] === undefined &&
			blocker.run.includes("externally pinned importer") &&
			/(?:^|\n)\s*exit 1\s*$/u.test(blocker.run),
		"source blocker must be the unconditional terminal fatal step",
	);
	assert.equal(source.includes("secrets."), false);
	assert.equal(source.includes("upload-artifact@"), false);
	assert.equal(source.includes("download-artifact@"), false);
	assert.equal(source.includes("pnpm install"), false);
	assert.equal(source.includes("fetch-account-acceptance-source"), false);

	for (const step of job.steps) {
		if (step.uses) {
			assert.match(step.uses, /@[a-f0-9]{40}$/u);
		}
	}
}

function mutateWorkflow(mutation) {
	const workflow = parse(workflowSource);
	mutation(workflow);
	return stringify(workflow);
}

function namedStep(job, name) {
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

function requireCondition(condition, message) {
	if (!condition) throw new Error(message);
}
