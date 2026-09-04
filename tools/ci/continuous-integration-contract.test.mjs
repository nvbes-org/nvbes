import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/ci.yml", "utf8");
const deploymentWorkflows = [
	"deploy-account.yml",
	"deploy-billing.yml",
	"deploy-email.yml",
	"deploy-identity.yml",
	"deploy-trust-risk.yml",
].map((name) =>
	readFileSync(`.github/workflows/${name}`, "utf8"),
);

test("continuous CI runs on every active delivery branch", () => {
	assert.match(workflow, /push:\n {4}branches: \['\*\*'\]/u);
	assert.match(workflow, /pull_request:\n {4}branches: \['main', 'dev'\]/u);
	assert.doesNotMatch(workflow, /staging/u);
	assert.match(workflow, /workflow_dispatch:/u);
	assert.match(workflow, /quality:/u);
	assert.match(
		workflow,
		/quality:[\s\S]*?runs-on: ubuntu-latest\n {4}timeout-minutes: (?:9\d|[1-9]\d{2,})/u,
	);
	assert.doesNotMatch(workflow, /rust-tests-scaleway-cache:/u);
});

test("continuous CI validates the active Email and closed Identity runtimes", () => {
	assert.match(workflow, /POSTGRES_DB: nvbes_email_test/u);
	assert.match(workflow, /job\.services\.postgres\.ports\['5432'\]/u);
	assert.match(workflow, /127\.0\.0\.1/u);
	assert.match(workflow, /pnpm test:pre-deploy/u);
	assert.match(workflow, /pnpm test/u);
	assert.match(workflow, /bash scripts\/test-email-worker-database\.sh/u);
	assert.match(
		workflow,
		/terraform -chdir=infrastructure\/stacks\/email\/production test/u,
	);
	assert.match(
		workflow,
		/node --test apps\/email-worker\/tests\/container-contract\.test\.mjs/u,
	);
	assert.match(
		workflow,
		/terraform -chdir=infrastructure\/environments\/identity-production validate/u,
	);
	assert.match(workflow, /pnpm nx run identity-service:test:container/u);
	assert.doesNotMatch(workflow, /identity-migration-checks/u);
	assert.doesNotMatch(workflow, /test-identity-service-migrations/u);
	assert.doesNotMatch(workflow, /pnpm test:unit/u);
});

test("continuous CI uses GitHub-hosted quality and self-hosted delivery runners", () => {
	assert.match(workflow, /runs-on: ubuntu-latest/u);
	for (const deploymentWorkflow of deploymentWorkflows) {
		assert.match(
			deploymentWorkflow,
			/runs-on: \[self-hosted, macOS, ARM64\]/u,
		);
	}
	assert.doesNotMatch(workflow, /Swatinem\/rust-cache@/u);
	assert.doesNotMatch(workflow, /^\s+cache: pnpm\s*$/mu);
	assert.match(
		workflow,
		/node --test[\s\S]*?tools\/ci\/continuous-integration-contract\.test\.mjs/u,
	);
});

test("continuous CI selects affected projects with an isolated Nx cache", () => {
	assert.match(workflow, /name: Configure Nx cache and affected scope/u);
	assert.match(workflow, /node tools\/ci\/nx-cache-manager\.mjs/u);
	assert.match(
		workflow,
		/if: steps\.nx-scope\.outputs\.rust-affected == 'true'/u,
	);
	assert.match(
		workflow,
		/if: steps\.nx-scope\.outputs\.typescript-projects != ''/u,
	);
	assert.match(workflow, /-t format:check,lint,typecheck,check/u);
	assert.match(workflow, /NVBES_CI_BASE_CANDIDATE/u);
	for (const scope of [
		"email",
		"identity",
		"account",
		"billing",
		"trust-risk",
		"platform",
	]) {
		assert.match(
			workflow,
			new RegExp(
				`if: steps\\.nx-scope\\.outputs\\.${scope}-affected == 'true'`,
				"u",
			),
		);
	}
});

test("continuous CI runs Rust tests once with the appropriate cache backend", () => {
	assert.match(workflow, /RUSTC_WRAPPER: sccache/u);
	assert.doesNotMatch(workflow, /rust-tests-scaleway-cache:/u);
	assert.match(
		workflow,
		/name: \$\{\{[\s\S]*?'production-ci-cache' \|\| 'branch-ci-cache' \}\}\n {6}deployment: false/u,
	);
	assert.match(
		workflow,
		/startsWith\(github\.ref, 'refs\/heads\/release\/'\)/u,
	);
	assert.match(workflow, /format\('branches\/\{0\}', github\.ref_name\)/u);
	assert.match(workflow, /cargo test --workspace --locked/u);
	assert.match(workflow, /name: Configure Rust compilation cache/u);
	assert.match(workflow, /sccache --start-server/u);
	assert.match(workflow, /name: Test Rust workspace/u);
	assert.match(workflow, /GITHUB_EVENT_NAME.*!=.*push/u);
	assert.match(workflow, /export SCCACHE_GHA_ENABLED=true/u);
	assert.equal(
		workflow.match(/cargo test --workspace --locked/gu)?.length,
		1,
		"the unified quality job must run the Rust workspace test exactly once",
	);
});
