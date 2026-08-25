import assert from "node:assert/strict";
import test from "node:test";
import { validateWorkflowContract } from "./finops-workflows-contract.mjs";

const safeShell = "bash --noprofile --norc -euo pipefail {0}";
const checkoutAction =
	"actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1";
const setupNodeAction =
	"actions/setup-node@820762786026740c76f36085b0efc47a31fe5020";
const workflowPath = ".github/workflows/deploy-email.yml";
const securityCommand = `node tools/security/check-ci-cd-security.mjs --workflow ${workflowPath}`;
const terraformEnvironmentPath = "infrastructure/environments/email-production";
const terraformValidation = `terraform -chdir=${terraformEnvironmentPath} validate`;
const contract = {
	job: "ci-test-gate",
	buildJob: "build-scan-sign",
	buildIf: `\${{ success() }}`,
	deployJob: "deploy-email",
	deployIf: `\${{ success() && github.ref == 'refs/heads/main' }}`,
	gateEnv: undefined,
	runsOn: ["self-hosted", "macOS", "ARM64"],
	shell: safeShell,
	terraformEnvironmentPath,
	terraformInitStepName: "Pre-deploy: Initialize email Terraform providers",
	terraformValidationStepName:
		"Pre-deploy: Validate isolated email Terraform stack",
	workflowPath,
};

const validCriticalSteps = `
      - uses: ${checkoutAction}
        with:
          fetch-depth: 0
          persist-credentials: false
      - uses: ${setupNodeAction}
        with:
          node-version: 24
      - name: CI/CD security gate
        run: ${securityCommand}
      - name: Enable pnpm
        run: |-
          corepack enable
          corepack prepare pnpm@11.18.0 --activate
      - name: Install locked dependencies
        run: pnpm install --frozen-lockfile --prefer-offline
      - name: Enforce FinOps contract
        run: pnpm check:finops
      - uses: hashicorp/setup-terraform@dfe3c3f87815947d99a8997f908cb6525fc44e9e
        with:
          terraform_version: 1.15.8
          terraform_wrapper: false
      - name: 'Pre-deploy: Initialize email Terraform providers'
        run: terraform -chdir=${terraformEnvironmentPath} init -backend=false -input=false
      - name: 'Pre-deploy: Validate isolated email Terraform stack'
        run: ${terraformValidation}`;

function workflowWithCriticalSteps(criticalSteps, defaultShell = safeShell) {
	return `
defaults:
  run:
    shell: ${defaultShell}
jobs:
  ci-test-gate:
    runs-on: [self-hosted, macOS, ARM64]
    steps:${criticalSteps}
  build-scan-sign:
    needs: ci-test-gate
    if: \${{ success() }}
    steps: []
  deploy-email:
    needs: [ci-test-gate, build-scan-sign]
    if: \${{ success() && github.ref == 'refs/heads/main' }}
    steps: []
`;
}

test("accepts the dedicated fail-closed baseline", () => {
	assert.doesNotThrow(() =>
		validateWorkflowContract(
			workflowWithCriticalSteps(validCriticalSteps),
			contract,
		),
	);
});

for (const command of [
	"node tools/security/check-ci-cd-security.mjs --workflow .github/workflows/ci.yml",
	`|-
          ${securityCommand}
          exit 0`,
]) {
	test("rejects a non-exact dedicated security gate", () => {
		const steps = validCriticalSteps.replace(securityCommand, command);
		assert.throws(
			() =>
				validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
			/must use the allowlisted gate step prefix/u,
		);
	});
}

for (const property of [
	"if: false",
	"continue-on-error: true",
	"shell: bash {0} || true",
]) {
	test(`rejects ${property} on the dedicated security gate`, () => {
		const steps = validCriticalSteps.replace(
			`        run: ${securityCommand}`,
			`        run: ${securityCommand}\n        ${property}`,
		);
		assert.throws(
			() =>
				validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
			/critical ci-test-gate steps must be unconditional and fail closed/u,
		);
	});
}

const unauthorizedPreGateSteps = [
	[
		"an obfuscated Terraform invocation",
		`      - run: |-
          t=terra
          f=form
          "$t$f" version`,
	],
	[
		"an additional action",
		"      - uses: actions/cache@0000000000000000000000000000000000000000",
	],
	["an additional command", "      - run: echo ready"],
];

for (const [scenario, step] of unauthorizedPreGateSteps) {
	test(`rejects ${scenario} before the FinOps gate`, () => {
		const steps = validCriticalSteps.replace(
			`      - uses: ${setupNodeAction}`,
			`${step}\n      - uses: ${setupNodeAction}`,
		);

		assert.throws(
			() =>
				validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
			/must use the allowlisted gate step prefix/u,
		);
	});
}

const disguisedGates = [
	[
		"after an early successful exit",
		`|
          exit 0
          pnpm check:finops`,
	],
	[
		"inside a function that is never called",
		`|
          finops_gate() {
            pnpm check:finops
          }`,
	],
	[
		"inside a false shell branch",
		`|
          if false; then
            pnpm check:finops
          fi`,
	],
];

for (const [scenario, run] of disguisedGates) {
	test(`rejects a FinOps command ${scenario}`, () => {
		const steps = validCriticalSteps.replace(
			"        run: pnpm check:finops",
			`        run: ${run}`,
		);

		assert.throws(
			() =>
				validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
			/must invoke pnpm check:finops exactly once/u,
		);
	});
}

test("rejects a step inserted between the locked install and FinOps gate", () => {
	const steps = validCriticalSteps.replace(
		"        run: pnpm check:finops",
		`      - run: echo ready
      - name: Enforce FinOps contract
        run: pnpm check:finops`,
	);

	assert.throws(
		() => validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
		/must use the allowlisted gate step prefix/u,
	);
});

test("rejects a workflow whose default shell neutralizes failures", () => {
	assert.throws(
		() =>
			validateWorkflowContract(
				workflowWithCriticalSteps(validCriticalSteps, "bash {0} || true"),
				contract,
			),
		/workflow defaults must exactly define the fail-closed shell/u,
	);
});

for (const command of ["terraform version", "echo terraform"]) {
	test(`rejects ${command} as the required Terraform validation`, () => {
		const steps = validCriticalSteps.replace(
			`        run: ${terraformValidation}`,
			`        run: ${command}`,
		);

		assert.throws(
			() =>
				validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
			/must use the allowlisted gate step prefix/u,
		);
	});
}

const criticalCommands = [
	["locked install", "pnpm install --frozen-lockfile --prefer-offline"],
	["FinOps gate", "pnpm check:finops"],
	["Terraform validation", terraformValidation],
];

for (const [stepName, command] of criticalCommands) {
	for (const property of [
		"if: false",
		"continue-on-error: true",
		"shell: bash {0} || true",
	]) {
		test(`rejects ${property} on the dedicated ${stepName} step`, () => {
			const steps = validCriticalSteps.replace(
				`run: ${command}`,
				`run: ${command}\n        ${property}`,
			);

			assert.throws(
				() =>
					validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
				/critical ci-test-gate steps must be unconditional and fail closed/u,
			);
		});
	}
}
