import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/ci.yml", "utf8");
const nxConfig = JSON.parse(readFileSync("nx.json", "utf8"));
const deploymentWorkflows = [
	"deploy-account.yml",
	"deploy-billing.yml",
	"deploy-email.yml",
	"deploy-identity.yml",
	"deploy-trust-risk.yml",
].map((name) => readFileSync(`.github/workflows/${name}`, "utf8"));

test("continuous CI runs once per pull request and on protected pushes", () => {
	assert.match(
		workflow,
		/push:\n {4}branches: \[main, dev, 'release\/\*\*'\]/u,
	);
	assert.match(workflow, /pull_request:\n {4}branches: \[main, dev\]/u);
	assert.doesNotMatch(workflow, /branches: \['\*\*'\]/u);
	assert.doesNotMatch(
		workflow,
		/github\.event\.pull_request\.head\.repo\.full_name/u,
	);
	assert.match(
		workflow,
		/github\.event\.pull_request\.number \|\| github\.ref/u,
	);
	assert.match(workflow, /cancel-in-progress: true/u);
	assert.match(workflow, /workflow_dispatch:/u);
	assert.match(workflow, /authorize-cache:/u);
	assert.match(
		workflow,
		/ref: \$\{\{ github\.event\.pull_request\.base\.sha \}\}/u,
	);
	assert.match(
		workflow,
		/node trusted-base\/tools\/ci\/authorize-pr-cache\.mjs/u,
	);
	assert.match(workflow, /needs\.authorize-cache\.outputs\.trusted == 'true'/u);
});

test("continuous CI scopes expensive runtimes after affected discovery", () => {
	const scope = workflow.indexOf("name: Configure affected scope");
	const rust = workflow.indexOf("uses: dtolnay/rust-toolchain@");
	const terraform = workflow.indexOf("uses: hashicorp/setup-terraform@");
	const postgres = workflow.indexOf("name: Start scoped PostgreSQL");
	assert.ok(scope > 0);
	assert.ok(rust > scope);
	assert.ok(terraform > scope);
	assert.ok(postgres > scope);
	assert.doesNotMatch(workflow, /^ {4}services:/mu);
	assert.match(workflow, /docker run --detach --name nvbes-ci-postgres/u);
	assert.match(workflow, /docker rm --force nvbes-ci-postgres/u);
	assert.match(
		workflow,
		/if: steps\.nx-scope\.outputs\.rust-affected == 'true'/u,
	);
	assert.match(
		workflow,
		/if: steps\.nx-scope\.outputs\.rust-required == 'true'/u,
	);
	assert.match(
		workflow,
		/if: steps\.nx-scope\.outputs\.terraform-affected == 'true' \|\| steps\.nx-scope\.outputs\.email-affected == 'true'/u,
	);
});

test("continuous CI validates affected targets without requiring a uniform target set", () => {
	assert.match(workflow, /node tools\/ci\/nx-cache-manager\.mjs/u);
	assert.match(workflow, /NVBES_CI_BASE_CANDIDATE/u);
	assert.match(
		workflow,
		/if: steps\.nx-scope\.outputs\.typescript-projects != ''/u,
	);
	assert.match(workflow, /-t format:check,lint,typecheck,check,test/u);
	assert.match(workflow, /--parallel=4/u);
	assert.match(workflow, /--outputStyle=static/u);
	assert.equal(
		nxConfig.pluginsConfig?.["@nx/js"]?.projectsAffectedByDependencyUpdates,
		"auto",
	);
});

test("continuous CI restores and publishes only the caches each scope consumes", () => {
	const pnpmRestore = workflow.indexOf("name: Restore pnpm cache");
	const install = workflow.indexOf("name: Install locked dependencies");
	const nxRestore = workflow.indexOf("name: Restore Nx task cache");
	const scope = workflow.indexOf("name: Configure affected scope");
	assert.ok(pnpmRestore > 0 && pnpmRestore < install);
	assert.ok(nxRestore > scope);
	assert.match(workflow, /scaleway-cache-manager\.mjs restore pnpm/u);
	assert.match(workflow, /scaleway-cache-manager\.mjs restore nx/u);
	assert.match(workflow, /mkdir -p "\$TF_PLUGIN_CACHE_DIR"/u);
	assert.match(workflow, /cache_kinds\+=\(cargo\)/u);
	assert.match(workflow, /cache_kinds\+=\(terraform\)/u);
	assert.match(workflow, /scaleway-cache-manager\.mjs save pnpm nx/u);
	assert.match(workflow, /scaleway-cache-manager\.mjs save cargo/u);
	assert.match(workflow, /scaleway-cache-manager\.mjs save terraform/u);
	assert.match(workflow, /if: success\(\) && github\.event_name == 'push'/u);
});

test("continuous CI runs affected Rust packages once and only for Rust inputs", () => {
	assert.match(workflow, /RUSTC_WRAPPER: sccache/u);
	assert.match(workflow, /name: Configure Rust compilation cache/u);
	assert.match(workflow, /name: Test affected Rust workspace/u);
	assert.match(workflow, /cargo fmt --all --check/u);
	assert.match(workflow, /node tools\/ci\/cargo-affected\.mjs/u);
	assert.match(workflow, /cargo_args\+=\(--package "\$package"\)/u);
	assert.equal(workflow.match(/cargo test --locked/gu)?.length, 1);
	assert.doesNotMatch(workflow, /cargo test --workspace/u);
	assert.match(workflow, /export SCCACHE_GHA_ENABLED=true/u);
});

test("continuous CI scopes database, Terraform, and container contracts independently", () => {
	for (const scope of ["account", "email"]) {
		assert.match(
			workflow,
			new RegExp(
				`steps\\.nx-scope\\.outputs\\.${scope}-database-affected`,
				"u",
			),
		);
	}
	for (const scope of [
		"account",
		"billing",
		"email",
		"identity",
		"platform",
		"trust-risk",
	]) {
		assert.match(
			workflow,
			new RegExp(
				`steps\\.nx-scope\\.outputs\\.${scope}-container-affected`,
				"u",
			),
		);
	}
	for (const scope of [
		"account",
		"billing",
		"email",
		"identity",
		"platform-operations",
		"trust-risk",
		"security-audit-archive",
	]) {
		assert.match(
			workflow,
			new RegExp(
				`steps\\.nx-scope\\.outputs\\.${scope}-terraform-affected`,
				"u",
			),
		);
	}
	assert.equal(
		workflow.match(/terraform .* init .* -lockfile=readonly/gu)?.length,
		9,
	);
});

test("continuous CI uses GitHub-hosted quality and self-hosted delivery runners", () => {
	assert.match(workflow, /runs-on: ubuntu-latest/u);
	assert.match(workflow, /timeout-minutes: 45/u);
	for (const deploymentWorkflow of deploymentWorkflows) {
		assert.match(deploymentWorkflow, /runs-on: \[self-hosted, macOS, ARM64\]/u);
	}
	assert.doesNotMatch(workflow, /Swatinem\/rust-cache@/u);
});
