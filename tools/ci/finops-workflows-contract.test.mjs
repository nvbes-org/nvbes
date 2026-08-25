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
const emailTerraformValidation =
	"terraform -chdir=infrastructure/environments/email-production validate";
const deployContract = {
	path: ".github/workflows/deploy-email.yml",
	job: "ci-test-gate",
	buildJob: "build-scan-sign",
	buildIf: `\${{ success() }}`,
	deployJob: "deploy-email",
	deployIf: `\${{ success() && github.ref == 'refs/heads/main' }}`,
	shell: safeShell,
	terraformValidation: emailTerraformValidation,
	workflowPath: ".github/workflows/deploy-email.yml",
};
const workflowContracts = [
	{
		path: ".github/workflows/ci.yml",
		job: "email-quality",
		shell: safeShell,
		terraformValidation: emailTerraformValidation,
		workflowPath: ".github/workflows/ci.yml",
	},
	deployContract,
	{
		...deployContract,
		path: ".github/workflows/deploy-trust-risk.yml",
		deployJob: "deploy-trust-risk",
		terraformValidation:
			"terraform -chdir=infrastructure/environments/trust-risk-production validate",
		workflowPath: ".github/workflows/deploy-trust-risk.yml",
	},
];

const validPrefix = `
      - uses: ${checkoutAction}
      - uses: ${setupNodeAction}
        with:
          node-version: 24
      - run: node tools/security/check-ci-cd-security.mjs --workflow .github/workflows/deploy-email.yml
      - run: |-
          corepack enable
          corepack prepare pnpm@11.18.0 --activate
      - run: ${lockedInstall}
      - run: pnpm check:finops`;
const validGateSteps = `${validPrefix}
      - run: ${emailTerraformValidation}`;
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
${property}    steps:${steps}
${downstream}`;
}

function withStepProperty(steps, command, property) {
	return steps.replace(
		`      - run: ${command}`,
		`      - run: ${command}\n        ${property}`,
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

test("accepts explicit false continue-on-error values on the gate and critical steps", () => {
	let steps = validGateSteps;
	for (const command of [
		lockedInstall,
		"pnpm check:finops",
		emailTerraformValidation,
	]) {
		steps = withStepProperty(steps, command, "continue-on-error: false");
	}
	assert.doesNotThrow(() =>
		validateWorkflowContract(
			deployWorkflow({ steps, gateProperty: "    continue-on-error: false" }),
			deployContract,
		),
	);
});

test("rejects a Terraform mention without a real Terraform command after the gate", () => {
	const steps = validGateSteps.replace(
		emailTerraformValidation,
		"echo terraform",
	);
	assert.throws(
		() => validateWorkflowContract(deployWorkflow({ steps }), deployContract),
		/Terraform validation must run after pnpm check:finops/u,
	);
});

test("rejects an invocation moved outside the gate even when a gate comment mentions it", () => {
	const steps = validGateSteps.replace(
		"      - run: pnpm check:finops",
		`      - run: |
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
			/must use the allowlisted pre-gate step prefix/u,
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
		/must use the allowlisted pre-gate step prefix/u,
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
