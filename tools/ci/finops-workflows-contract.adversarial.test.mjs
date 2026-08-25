import assert from "node:assert/strict";
import test from "node:test";
import { validateWorkflowContract } from "./finops-workflows-contract.mjs";

const safeShell = "bash --noprofile --norc -euo pipefail {0}";
const terraformValidation =
	"terraform -chdir=infrastructure/environments/email-production validate";
const contract = {
	job: "ci-test-gate",
	buildJob: "build-scan-sign",
	buildIf: `\${{ success() }}`,
	deployJob: "deploy-email",
	deployIf: `\${{ success() && github.ref == 'refs/heads/main' }}`,
	shell: safeShell,
	terraformValidation,
};

const validCriticalSteps = `
      - run: pnpm install --frozen-lockfile
      - run: pnpm check:finops
      - run: ${terraformValidation}`;

function workflowWithCriticalSteps(criticalSteps, defaultShell = safeShell) {
	return `
defaults:
  run:
    shell: ${defaultShell}
jobs:
  ci-test-gate:
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
		const steps = `
      - run: pnpm install --frozen-lockfile
      - run: ${run}
      - run: ${terraformValidation}`;

		assert.throws(
			() =>
				validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
			/must invoke pnpm check:finops exactly once/u,
		);
	});
}

test("rejects a step inserted between the locked install and FinOps gate", () => {
	const steps = `
      - run: pnpm install --frozen-lockfile
      - run: echo ready
      - run: pnpm check:finops
      - run: ${terraformValidation}`;

	assert.throws(
		() => validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
		/locked pnpm install must run before pnpm check:finops/u,
	);
});

test("rejects a workflow whose default shell neutralizes failures", () => {
	assert.throws(
		() =>
			validateWorkflowContract(
				workflowWithCriticalSteps(validCriticalSteps, "bash {0} || true"),
				contract,
			),
		/required fail-closed default shell/u,
	);
});

for (const command of ["terraform version", "echo terraform"]) {
	test(`rejects ${command} as the required Terraform validation`, () => {
		const steps = `
      - run: pnpm install --frozen-lockfile
      - run: pnpm check:finops
      - run: ${command}`;

		assert.throws(
			() =>
				validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
			/Terraform validation must run after pnpm check:finops/u,
		);
	});
}

const criticalCommands = [
	["locked install", "pnpm install --frozen-lockfile"],
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
				`- run: ${command}`,
				`- run: ${command}\n        ${property}`,
			);

			assert.throws(
				() =>
					validateWorkflowContract(workflowWithCriticalSteps(steps), contract),
				/critical ci-test-gate steps must be unconditional and fail closed/u,
			);
		});
	}
}
