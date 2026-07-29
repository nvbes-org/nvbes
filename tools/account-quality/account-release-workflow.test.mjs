import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { parse, stringify } from "yaml";

const workflowPath = ".github/workflows/account-release.yml";
const workflowSource = await readFile(workflowPath, "utf8");
const registry = JSON.parse(
	await readFile("docs/security/ci-cd-security-controls.json", "utf8"),
);
const expression = (value) => `\${{ ${value} }}`;

test("release workflow enforces the protected exact-SHA trust boundary", () => {
	assert.doesNotThrow(() => validateAccountReleaseWorkflow(workflowSource));
});

test("release workflow rejects dispatch inputs", () => {
	assert.throws(
		() =>
			validateAccountReleaseWorkflow(
				mutateWorkflow((workflow) => {
					workflow.on.workflow_dispatch = {
						inputs: { release: { type: "string" } },
					};
				}),
			),
		/workflow_dispatch must not accept inputs/u,
	);
});

test("release workflow rejects environment drift", () => {
	assert.throws(
		() =>
			validateAccountReleaseWorkflow(
				mutateWorkflow((workflow) => {
					workflow.jobs["release-account"].environment.name = "production";
				}),
			),
		/release job must target production-account/u,
	);
});

test("release workflow rejects a mutable release selector", () => {
	assert.throws(
		() =>
			validateAccountReleaseWorkflow(
				mutateWorkflow((workflow) => {
					workflow.jobs["release-account"].env.NVBES_RELEASE_SHA = expression(
						"vars.NVBES_RELEASE_SHA",
					);
				}),
			),
		/release must be bound to github.sha/u,
	);
});

test("release workflow rejects injected trust", () => {
	assert.throws(
		() =>
			validateAccountReleaseWorkflow(
				mutateWorkflow((workflow) => {
					workflow.jobs[
						"release-account"
					].env.ACCOUNT_ACCEPTANCE_TRUSTED_KEY_ID = "operator-input";
				}),
			),
		/protected vars/u,
	);
});

test("release workflow rejects non-SHA evidence selection", () => {
	assert.throws(
		() =>
			validateAccountReleaseWorkflow(
				mutateWorkflow((workflow) => {
					namedStep(
						workflow,
						"Download exact-SHA acceptance evidence",
					).with.name = "account-acceptance-latest";
				}),
			),
		/exact github SHA/u,
	);
});

test("release workflow keeps verification, download, gate and upload fatal", () => {
	for (const stepName of [
		"Validate immutable Account RC tag",
		"Validate exact release and evidence source",
		"Download exact-SHA acceptance evidence",
		"Run exact production release gate",
		"Upload immutable release report",
	]) {
		assert.throws(
			() =>
				validateAccountReleaseWorkflow(
					mutateWorkflow((workflow) => {
						namedStep(workflow, stepName)["continue-on-error"] = true;
					}),
				),
			/critical release steps must fail the job/u,
		);
	}
	assert.throws(
		() =>
			validateAccountReleaseWorkflow(
				mutateWorkflow((workflow) => {
					workflow.jobs["release-account"]["continue-on-error"] = true;
				}),
			),
		/critical release steps must fail the job/u,
	);
});

function validateAccountReleaseWorkflow(source) {
	const workflow = parse(source);
	assert.deepEqual(
		Object.keys(workflow.on ?? {}),
		["workflow_dispatch"],
		"release must only support workflow_dispatch",
	);
	assert(
		workflow.on.workflow_dispatch == null ||
			Object.keys(workflow.on.workflow_dispatch).length === 0,
		"workflow_dispatch must not accept inputs",
	);
	assert(
		!source.includes("inputs."),
		"release trust must never come from workflow inputs",
	);
	assert.deepEqual(workflow.permissions, { actions: "read", contents: "read" });
	assert.deepEqual(workflow.concurrency, {
		"cancel-in-progress": false,
		group: "account-production-release",
	});
	assert.equal(
		workflow.defaults?.run?.shell,
		"bash --noprofile --norc -euo pipefail {0}",
	);
	assert.deepEqual(Object.keys(workflow.jobs ?? {}), ["release-account"]);

	const job = workflow.jobs["release-account"];
	assert.equal(
		job.environment?.name,
		"production-account",
		"release job must target production-account",
	);
	assert.equal(
		job.environment?.url,
		expression("vars.NVBES_PRODUCTION_WEB_BASE_URL"),
	);
	assert.equal(job["runs-on"], "ubuntu-latest");
	assert.equal(job["timeout-minutes"], 180);
	assert.equal(
		job["continue-on-error"],
		undefined,
		"critical release steps must fail the job",
	);
	assert.equal(job.env.RELEASE_APPROVED, "production");
	assert.equal(
		job.env.NVBES_RELEASE_SHA,
		expression("github.sha"),
		"release must be bound to github.sha",
	);
	assert.equal(job.env.NVBES_RELEASE_REF, expression("github.ref"));
	assert.equal(job.env.NVBES_RELEASE_REF_NAME, expression("github.ref_name"));
	assert.equal(job.env.NVBES_RELEASE_REF_TYPE, expression("github.ref_type"));
	assert.equal(
		job.env.DATABASE_URL,
		"postgres://postgres:postgres@127.0.0.1:5432/nvbes_test",
	);
	assert.equal(job.env.NVBES_DATABASE_URL, job.env.DATABASE_URL);
	assert.equal(
		job.env.NVBES_ALLOW_DESTRUCTIVE_TEST_DATABASE,
		"account-quality-v1",
	);
	assert.equal(job.env.NVBES_ENV, "test");
	assert.equal(job.env.NVBES_REDIS_URL, "redis://127.0.0.1:6379/15");
	assert.equal(job.services.postgres.image, "postgres:17-alpine");
	assert.equal(job.services.postgres.env.POSTGRES_DB, "nvbes_test");
	assert.deepEqual(job.services.postgres.ports, ["5432:5432"]);
	assert.match(
		job.services.postgres.options,
		/pg_isready -U postgres -d nvbes_test/u,
	);
	assert.equal(job.services.redis.image, "redis:7-alpine");
	assert.deepEqual(job.services.redis.ports, ["6379:6379"]);
	assert.match(job.services.redis.options, /redis-cli ping/u);

	for (const variable of [
		"ACCOUNT_PRODUCTION_DENIED_ORIGINS",
		"ACCOUNT_ACCEPTANCE_TRUSTED_KEY_ID",
		"ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_SHA256",
		"ACCOUNT_DEPLOYMENT_EXPECTED_COMPONENT_UIDS",
		"ACCOUNT_DEPLOYMENT_TRUSTED_KEY_ID",
		"ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_SHA256",
		"NVBES_FAPI_CONFORMANCE_EVIDENCE_SHA256",
		"NVBES_FAPI_HIGH_ASSURANCE_ENABLED",
		"NVBES_PRODUCTION_ALLOWED_ORIGINS",
		"NVBES_PRODUCTION_API_BASE_URL",
		"NVBES_PRODUCTION_WEB_BASE_URL",
		"NVBES_STAGING_ACCOUNT_EMAIL",
		"NVBES_STAGING_ALLOWED_API_ORIGINS",
		"NVBES_STAGING_ALLOWED_WEB_ORIGINS",
		"NVBES_STAGING_API_BASE_URL",
		"NVBES_STAGING_WEB_BASE_URL",
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
	assert.equal(actionStep(job, "actions/setup-node").with["node-version"], 24);
	assert.equal(actionStep(job, "actions/setup-node").with.cache, "pnpm");
	assert.match(namedStep(job, "Enable pnpm").run, /pnpm@11\.17\.0 --activate/u);
	assert.match(
		namedStep(job, "Install xmlsec system dependencies").run,
		/libxmlsec1-dev libxmlsec1-openssl xmlsec1/u,
	);
	assert.equal(
		actionStep(job, "dtolnay/rust-toolchain").uses,
		"dtolnay/rust-toolchain@4cda84d5c5c54efe2404f9d843567869ab1699d4",
	);
	assert.equal(
		actionStep(job, "Swatinem/rust-cache").uses,
		"Swatinem/rust-cache@e18b497796c12c097a38f9edb9d0641fb99eee32",
	);

	const sourceProof = namedStep(
		job,
		"Validate exact release and evidence source",
	);
	for (const proof of [
		"run.head_branch !== expectedTag",
		"run.head_sha !== release",
		'run.conclusion !== "success"',
		"run.repository?.full_name !== repository",
		'run.path !== ".github/workflows/account-acceptance-ingest.yml"',
	]) {
		assert.ok(sourceProof.run.includes(proof), `missing source proof ${proof}`);
	}
	assert.equal(
		sourceProof.env.ACCOUNT_ACCEPTANCE_EVIDENCE_RUN_ID,
		expression("vars.ACCOUNT_ACCEPTANCE_EVIDENCE_RUN_ID"),
	);
	assert.equal(
		sourceProof.env.EXPECTED_RELEASE_TAG,
		expression("github.ref_name"),
	);

	const download = namedStep(job, "Download exact-SHA acceptance evidence");
	assert.equal(
		download.uses,
		"actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c",
	);
	assert.equal(
		download.with.name,
		`account-acceptance-${expression("github.sha")}`,
		"evidence artifact must bind the exact github SHA",
	);
	assert.equal(
		download.with["run-id"],
		expression("vars.ACCOUNT_ACCEPTANCE_EVIDENCE_RUN_ID"),
	);
	assert.equal(download.with["github-token"], expression("github.token"));
	assert.equal(download.with.repository, expression("github.repository"));

	const trust = namedStep(job, "Materialize protected trust roots");
	assert.deepEqual(trust.env, {
		ACCEPTANCE_TRUSTED_PUBLIC_KEY_PEM: expression(
			"secrets.ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_PEM",
		),
		DEPLOYMENT_TRUSTED_PUBLIC_KEY_PEM: expression(
			"secrets.ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_PEM",
		),
	});
	const gate = namedStep(job, "Run exact production release gate");
	assert.match(gate.run, /scripts\/release-gate\.sh production/u);
	assert.deepEqual(gate.env, {
		NVBES_STAGING_ACCOUNT_PASSWORD: expression(
			"secrets.NVBES_STAGING_ACCOUNT_PASSWORD",
		),
	});
	const browser = namedStep(job, "Install Account Chromium browser");
	assert.equal(
		browser.run,
		"pnpm --dir apps/account-web exec playwright install --with-deps chromium",
	);
	assert.ok(job.steps.indexOf(browser) < job.steps.indexOf(gate));
	const downloadIndex = job.steps.indexOf(
		namedStep(job, "Download exact-SHA acceptance evidence"),
	);
	const trustIndex = job.steps.indexOf(
		namedStep(job, "Materialize protected trust roots"),
	);
	const installIndex = job.steps.indexOf(
		namedStep(job, "Install locked dependencies"),
	);
	assert.ok(installIndex < downloadIndex);
	assert.ok(job.steps.indexOf(browser) < downloadIndex);
	assert.ok(downloadIndex < trustIndex);

	const report = namedStep(job, "Write immutable release report");
	assert.equal(report.if, "always()");
	assert.match(
		report.run,
		/evidenceArtifact: `account-acceptance-\$\{release\}`/u,
	);
	assert.match(report.run, /release,/u);
	const upload = namedStep(job, "Upload immutable release report");
	assert.equal(upload.if, "always()");
	assert.equal(
		upload.with.name,
		`account-release-${expression("github.sha")}-${expression("github.run_id")}`,
	);
	assert.equal(upload.with["if-no-files-found"], "error");
	assert.equal(upload.with["retention-days"], 90);

	for (const stepName of [
		"Validate immutable Account RC tag",
		"Validate exact release and evidence source",
		"Download exact-SHA acceptance evidence",
		"Run exact production release gate",
		"Upload immutable release report",
	]) {
		assert.equal(
			namedStep(job, stepName)["continue-on-error"],
			undefined,
			"critical release steps must fail the job",
		);
	}

	for (const step of job.steps) {
		if (step.uses) {
			assert.match(
				step.uses,
				/@[a-f0-9]{40}$/u,
				`${step.uses} is not SHA-pinned`,
			);
		}
	}
	const referencedSecrets = [...source.matchAll(/secrets\.([A-Z0-9_]+)/gu)].map(
		(match) => match[1],
	);
	assert.deepEqual([...new Set(referencedSecrets)].sort(), [
		"ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_PEM",
		"ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_PEM",
		"NVBES_STAGING_ACCOUNT_PASSWORD",
	]);
	for (const secret of referencedSecrets) {
		assert.ok(
			registry.allowedSecrets.includes(secret),
			`${secret} is not allowlisted`,
		);
	}
	assert.ok(
		registry.allowedActions.includes(download.uses),
		"download-artifact pin is not allowlisted",
	);

	const securityGateIndex = job.steps.indexOf(
		namedStep(job, "CI/CD security gate"),
	);
	assert.ok(securityGateIndex >= 0 && securityGateIndex < installIndex);
}

function mutateWorkflow(mutation) {
	const workflow = parse(workflowSource);
	mutation(workflow);
	return stringify(workflow);
}

function namedStep(jobOrWorkflow, name) {
	const job = jobOrWorkflow.steps
		? jobOrWorkflow
		: jobOrWorkflow.jobs["release-account"];
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
