import assert from "node:assert/strict";
import test from "node:test";
import { validateWorkflowContract } from "./finops-workflows-contract.mjs";

const workflowPath = ".github/workflows/deploy-email.yml";
const terraformValidation =
	"terraform -chdir=infrastructure/environments/email-production validate";
const contract = {
	job: "ci-test-gate",
	gateEnv: undefined,
	runsOn: ["self-hosted", "macOS", "ARM64"],
	shell: "bash --noprofile --norc -euo pipefail {0}",
	terraformValidationStepName:
		"Pre-deploy: Validate isolated email Terraform stack",
	terraformValidation,
	workflowPath,
};

const validWorkflow = `
defaults:
  run:
    shell: bash --noprofile --norc -euo pipefail {0}
jobs:
  ci-test-gate:
    runs-on: [self-hosted, macOS, ARM64]
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1
        with:
          fetch-depth: 0
          persist-credentials: false
      - uses: actions/setup-node@820762786026740c76f36085b0efc47a31fe5020
        with:
          node-version: 24
      - name: CI/CD security gate
        run: node tools/security/check-ci-cd-security.mjs --workflow ${workflowPath}
      - name: Enable pnpm
        run: |-
          corepack enable
          corepack prepare pnpm@11.18.0 --activate
      - name: Install locked dependencies
        run: pnpm install --frozen-lockfile --prefer-offline
      - name: Enforce FinOps contract
        run: pnpm check:finops
      - name: 'Pre-deploy: Validate isolated email Terraform stack'
        run: ${terraformValidation}
`;

test("accepts the exact gate execution context", () => {
	assert.doesNotThrow(() => validateWorkflowContract(validWorkflow, contract));
});

const contextMutations = [
	[
		"BASH_ENV on the FinOps step",
		"        run: pnpm check:finops",
		`        run: pnpm check:finops
        env:
          BASH_ENV: /tmp/override`,
	],
	[
		"workflow BASH_ENV",
		"jobs:",
		`env:
  BASH_ENV: /tmp/override
jobs:`,
	],
	[
		"a critical working directory",
		"        run: pnpm check:finops",
		`        run: pnpm check:finops
        working-directory: /tmp`,
	],
	[
		"an extra setup-node option",
		"          node-version: 24",
		`          node-version: 24
          cache: pnpm`,
	],
];

for (const [scenario, needle, replacement] of contextMutations) {
	test(`rejects ${scenario}`, () => {
		assert.throws(() =>
			validateWorkflowContract(
				validWorkflow.replace(needle, replacement),
				contract,
			),
		);
	});
}

for (const variable of ["BASH_ENV", "PATH", "NODE_OPTIONS"]) {
	test(`rejects gate job ${variable}`, () => {
		const workflow = validWorkflow.replace(
			"    timeout-minutes: 45",
			`    timeout-minutes: 45
    env:
      ${variable}: /tmp/override`,
		);
		assert.throws(() => validateWorkflowContract(workflow, contract));
	});
}

for (const option of ["repository", "ref", "path"]) {
	test(`rejects checkout ${option}`, () => {
		const workflow = validWorkflow.replace(
			"          persist-credentials: false",
			`          persist-credentials: false
          ${option}: attacker/repository`,
		);
		assert.throws(() => validateWorkflowContract(workflow, contract));
	});
}
