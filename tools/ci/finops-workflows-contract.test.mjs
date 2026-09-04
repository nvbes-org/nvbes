import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { validateWorkflowContract } from "./finops-workflows-contract.mjs";

const safeShell = "bash --noprofile --norc -euo pipefail {0}";
const checkoutAction =
	"actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1";
const setupNodeAction =
	"actions/setup-node@820762786026740c76f36085b0efc47a31fe5020";
const lockedInstall = "pnpm install --frozen-lockfile --prefer-offline";
const emailTerraformEnvironmentPath =
	"infrastructure/environments/email-production";
const emailTerraformValidation = `terraform -chdir=${emailTerraformEnvironmentPath} validate`;
const deployContract = {
	path: ".github/workflows/deploy-email.yml",
	job: "ci-test-gate",
	buildJob: "build-scan-sign",
	buildIf: `\${{ success() }}`,
	deployJob: "deploy-email",
	deployIf: `\${{ success() && github.ref == 'refs/heads/main' }}`,
	gateEnv: undefined,
	runsOn: ["self-hosted", "macOS", "ARM64"],
	shell: safeShell,
	terraformEnvironmentPath: emailTerraformEnvironmentPath,
	terraformInitStepName: "Pre-deploy: Initialize email Terraform providers",
	terraformValidationStepName:
		"Pre-deploy: Validate isolated email Terraform stack",
	workflowPath: ".github/workflows/deploy-email.yml",
};
const workflowContracts = [
	{
		path: ".github/workflows/ci.yml",
		job: "quality",
		allowedIf:
			"github.event_name != 'pull_request' || github.event.pull_request.head.repo.full_name != github.repository",
		dependencyCacheStep: {
			env: {
				AWS_ACCESS_KEY_ID: `\${{ secrets.SCW_CI_CACHE_ACCESS_KEY }}`,
				AWS_SECRET_ACCESS_KEY: `\${{ secrets.SCW_CI_CACHE_SECRET_KEY }}`,
			},
			name: "Restore central dependency caches",
			run: 'mkdir -p "$TF_PLUGIN_CACHE_DIR"\nnode tools/ci/scaleway-cache-manager.mjs restore pnpm cargo terraform\n',
		},
		gateEnv: {
			CARGO_INCREMENTAL: "0",
			CI: "true",
			NVBES_ENV: "ci",
			RUSTFLAGS: "-C debuginfo=0",
			RUSTC_WRAPPER: "sccache",
			SCCACHE_BUCKET: `\${{ vars.SCW_CI_CACHE_BUCKET }}`,
			SCCACHE_ENDPOINT: `\${{ vars.SCW_CI_CACHE_S3_ENDPOINT }}`,
			SCCACHE_REGION: `\${{ vars.SCW_CI_CACHE_REGION }}`,
			SCCACHE_S3_ENABLE_VIRTUAL_HOST_STYLE: "true",
			SCCACHE_S3_USE_SSL: "true",
			TF_PLUGIN_CACHE_DIR: "/tmp/nvbes-terraform-plugin-cache",
		},
		runsOn: "ubuntu-latest",
		shell: safeShell,
		terraformEnvironmentPath: emailTerraformEnvironmentPath,
		terraformInitStepName: "Initialize isolated email Terraform providers",
		terraformValidationStepName: "Validate isolated email Terraform stack",
		workflowPath: ".github/workflows/ci.yml",
	},
];

const validPrefix = `
      - uses: ${checkoutAction}
        with:
          fetch-depth: 0
          persist-credentials: false
      - uses: ${setupNodeAction}
        with:
          node-version: 24
      - name: CI/CD security gate
        run: node tools/security/check-ci-cd-security.mjs --workflow .github/workflows/deploy-email.yml
      - name: Enable pnpm
        run: |-
          corepack enable
          corepack prepare pnpm@11.18.0 --activate
      - name: Install locked dependencies
        run: ${lockedInstall}
      - name: Enforce FinOps contract
        run: pnpm check:finops`;
const validGateSteps = `${validPrefix}
      - uses: hashicorp/setup-terraform@dfe3c3f87815947d99a8997f908cb6525fc44e9e
        with:
          terraform_version: 1.15.8
          terraform_wrapper: false
      - name: 'Pre-deploy: Initialize email Terraform providers'
        run: terraform -chdir=${emailTerraformEnvironmentPath} init -backend=false -input=false
      - name: 'Pre-deploy: Validate isolated email Terraform stack'
        run: ${emailTerraformValidation}`;
const validDeployNeeds = `
  build-scan-sign:
    needs: ci-test-gate
    if: \${{ success() }}
    steps: []
  deploy-email:
    needs: [ci-test-gate, build-scan-sign]
    if: \${{ success() && github.ref == 'refs/heads/main' }}
    steps: []`;

function deployWorkflow({
	steps = validGateSteps,
	gateProperty = "",
	downstream = validDeployNeeds,
} = {}) {
	const property = gateProperty.length > 0 ? `${gateProperty}\n` : "";
	return `
defaults:
  run:
    shell: ${safeShell}
jobs:
  ci-test-gate:
${property}    runs-on: [self-hosted, macOS, ARM64]
    steps:${steps}
${downstream}`;
}

function withStepProperty(steps, command, property) {
	return steps.replace(
		`        run: ${command}`,
		`        run: ${command}\n        ${property}`,
	);
}

function insertBeforeSetupNode(step) {
	return validGateSteps.replace(
		`      - uses: ${setupNodeAction}`,
		`${step}\n      - uses: ${setupNodeAction}`,
	);
}

for (const [scenario, gateProperty] of [
	["continue-on-error", "    continue-on-error: true"],
	["a false condition", "    if: false"],
	["any job-level condition", "    if: true"],
]) {
	test(`rejects a FinOps gate job with ${scenario}`, () => {
		assert.throws(
			() =>
				validateWorkflowContract(
					deployWorkflow({ gateProperty }),
					deployContract,
				),
			/ci-test-gate must be unconditional and fail closed/u,
		);
	});
}

for (const [scenario, command, property] of [
	["locked install", lockedInstall, "continue-on-error: true"],
	["conditional locked install", lockedInstall, "if: false"],
	["FinOps gate", "pnpm check:finops", "continue-on-error: true"],
	["conditional FinOps gate", "pnpm check:finops", "if: false"],
	["Terraform validation", emailTerraformValidation, "continue-on-error: true"],
	["conditional Terraform validation", emailTerraformValidation, "if: false"],
]) {
	test(`rejects a fail-open ${scenario} step`, () => {
		const steps = withStepProperty(validGateSteps, command, property);
		assert.throws(
			() => validateWorkflowContract(deployWorkflow({ steps }), deployContract),
			/critical ci-test-gate steps must be unconditional and fail closed/u,
		);
	});
}

test("rejects explicit false continue-on-error values on critical steps", () => {
	let steps = validGateSteps;
	for (const command of [
		lockedInstall,
		"pnpm check:finops",
		emailTerraformValidation,
	]) {
		steps = withStepProperty(steps, command, "continue-on-error: false");
	}
	assert.throws(
		() =>
			validateWorkflowContract(
				deployWorkflow({ steps, gateProperty: "    continue-on-error: false" }),
				deployContract,
			),
		/must use the allowlisted gate step prefix/u,
	);
});

test("rejects a Terraform mention without a real Terraform command after the gate", () => {
	const steps = validGateSteps.replace(
		emailTerraformValidation,
		"echo terraform",
	);
	assert.throws(
		() => validateWorkflowContract(deployWorkflow({ steps }), deployContract),
		/must use the allowlisted gate step prefix/u,
	);
});

test("rejects an invocation moved outside the gate even when a gate comment mentions it", () => {
	const steps = validGateSteps.replace(
		"        run: pnpm check:finops",
		`        run: |
          # pnpm check:finops`,
	);
	const downstream = `
  other:
    steps:
      - run: pnpm check:finops
${validDeployNeeds}`;
	assert.throws(
		() =>
			validateWorkflowContract(
				deployWorkflow({ steps, downstream }),
				deployContract,
			),
		/ci-test-gate must invoke pnpm check:finops exactly once/u,
	);
});

const preGateTerraformCommands = [
	["an inline Terraform command", "terraform plan"],
	["a chained directory change", "cd infrastructure && terraform plan"],
	["the command builtin", "command terraform plan"],
	["the env utility", "env TF_IN_AUTOMATION=true terraform plan"],
	["a shell conditional", "if true; then terraform plan; fi"],
];

for (const [scenario, command] of preGateTerraformCommands) {
	test(`rejects Terraform before the FinOps gate through ${scenario}`, () => {
		const steps = insertBeforeSetupNode(`      - run: ${command}`);
		assert.throws(
			() => validateWorkflowContract(deployWorkflow({ steps }), deployContract),
			/must use the allowlisted gate step prefix/u,
		);
	});
}

test("rejects a locked install whose failure is ignored", () => {
	const steps = validGateSteps.replace(
		lockedInstall,
		`${lockedInstall} || true`,
	);
	assert.throws(
		() => validateWorkflowContract(deployWorkflow({ steps }), deployContract),
		/must use the allowlisted gate step prefix/u,
	);
});

const downstreamFailures = [
	[
		"a deploy workflow whose build job bypasses the gate",
		`\n  build-scan-sign:\n    steps: []\n  deploy-email:\n    needs: [ci-test-gate, build-scan-sign]\n    steps: []`,
		/build-scan-sign must need ci-test-gate/u,
	],
	[
		"a build job that runs regardless of gate failure",
		validDeployNeeds.replace(`\${{ success() }}`, `\${{ always() }}`),
		/build-scan-sign must use the required success condition/u,
	],
	[
		"a deploy job that runs regardless of dependency failure",
		validDeployNeeds.replace(
			`\${{ success() && github.ref == 'refs/heads/main' }}`,
			`\${{ always() }}`,
		),
		/deploy-email must use the required success and branch condition/u,
	],
];

for (const [scenario, downstream, error] of downstreamFailures) {
	test(`rejects ${scenario}`, () => {
		assert.throws(
			() =>
				validateWorkflowContract(
					deployWorkflow({ downstream }),
					deployContract,
				),
			error,
		);
	});
}

for (const contract of workflowContracts) {
	test(`${contract.path} enforces FinOps before infrastructure validation`, () => {
		validateWorkflowContract(readFileSync(contract.path, "utf8"), contract);
	});
}
