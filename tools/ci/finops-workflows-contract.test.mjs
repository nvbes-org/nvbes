import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
import { validateWorkflowContract } from "./finops-workflows-contract.mjs";

const workflowContracts = [
	{ path: ".github/workflows/ci.yml", job: "email-quality" },
	{
		path: ".github/workflows/deploy-email.yml",
		job: "ci-test-gate",
		buildJob: "build-scan-sign",
		buildIf: `\${{ success() }}`,
		deployJob: "deploy-email",
		deployIf: `\${{ success() && github.ref == 'refs/heads/main' }}`,
	},
	{
		path: ".github/workflows/deploy-trust-risk.yml",
		job: "ci-test-gate",
		buildJob: "build-scan-sign",
		buildIf: `\${{ success() }}`,
		deployJob: "deploy-trust-risk",
		deployIf: `\${{ success() && github.ref == 'refs/heads/main' }}`,
	},
];

const validDeployNeeds = `
  build-scan-sign:
    needs: ci-test-gate
    if: \${{ success() }}
    steps: []
  deploy-email:
    needs: [ci-test-gate, build-scan-sign]
    if: \${{ success() && github.ref == 'refs/heads/main' }}
    steps: []
`;

function deployWorkflowWithGate(gate) {
	return `
jobs:
  ci-test-gate:
${gate}
${validDeployNeeds}`;
}

const failOpenGateJobs = [
	["continue-on-error", "    continue-on-error: true"],
	["a false condition", "    if: false"],
	["any job-level condition", "    if: true"],
];

for (const [scenario, gateProperty] of failOpenGateJobs) {
	test(`rejects a FinOps gate job with ${scenario}`, () => {
		const workflow = deployWorkflowWithGate(`${gateProperty}
    steps:
      - run: pnpm install --frozen-lockfile
      - run: pnpm check:finops
      - run: terraform validate`);

		assert.throws(
			() => validateWorkflowContract(workflow, workflowContracts[1]),
			/ci-test-gate must be unconditional and fail closed/u,
		);
	});
}

const failOpenCriticalSteps = [
	["locked install", 0, "continue-on-error: true"],
	["conditional locked install", 0, "if: false"],
	["FinOps gate", 1, "continue-on-error: true"],
	["conditional FinOps gate", 1, "if: false"],
	["Terraform validation", 2, "continue-on-error: true"],
	["conditional Terraform validation", 2, "if: false"],
];

for (const [
	scenario,
	criticalStepIndex,
	failOpenProperty,
] of failOpenCriticalSteps) {
	test(`rejects a fail-open ${scenario} step`, () => {
		const commands = [
			"pnpm install --frozen-lockfile",
			"pnpm check:finops",
			"terraform validate",
		];
		const steps = commands
			.map((command, index) => {
				const property =
					index === criticalStepIndex ? `\n        ${failOpenProperty}` : "";
				return `      - run: ${command}${property}`;
			})
			.join("\n");
		const workflow = deployWorkflowWithGate(`    steps:\n${steps}`);

		assert.throws(
			() => validateWorkflowContract(workflow, workflowContracts[1]),
			/critical ci-test-gate steps must be unconditional and fail closed/u,
		);
	});
}

test("accepts explicit false continue-on-error values on the gate and critical steps", () => {
	const workflow = deployWorkflowWithGate(`    continue-on-error: false
    steps:
      - run: pnpm install --frozen-lockfile
        continue-on-error: false
      - run: pnpm check:finops
        continue-on-error: false
      - run: terraform validate
        continue-on-error: false`);

	assert.doesNotThrow(() =>
		validateWorkflowContract(workflow, workflowContracts[1]),
	);
});

test("rejects a Terraform mention without a real Terraform command after the gate", () => {
	const workflow = deployWorkflowWithGate(`    steps:
      - run: pnpm install --frozen-lockfile
      - run: pnpm check:finops
      - run: echo terraform`);

	assert.throws(
		() => validateWorkflowContract(workflow, workflowContracts[1]),
		/Terraform validation must run after pnpm check:finops/u,
	);
});

test("rejects an invocation moved outside the gate even when a gate comment mentions it", () => {
	const workflow = `
jobs:
  ci-test-gate:
    steps:
      - run: pnpm install --frozen-lockfile
      - run: |
          # pnpm check:finops
          terraform -chdir=. validate
  other:
    steps:
      - run: pnpm check:finops
${validDeployNeeds}`;

	assert.throws(
		() => validateWorkflowContract(workflow, workflowContracts[1]),
		/ci-test-gate must invoke pnpm check:finops exactly once/u,
	);
});

test("rejects an inline Terraform command before the FinOps gate", () => {
	const workflow = `
jobs:
  ci-test-gate:
    steps:
      - run: pnpm install --frozen-lockfile
      - run: terraform plan
      - run: pnpm check:finops
      - run: terraform -chdir=. validate
${validDeployNeeds}`;

	assert.throws(
		() => validateWorkflowContract(workflow, workflowContracts[1]),
		/Terraform command must not run before pnpm check:finops/u,
	);
});

const indirectTerraformCommands = [
	["a chained directory change", "cd infrastructure && terraform plan"],
	["the command builtin", "command terraform plan"],
	["the env utility", "env TF_IN_AUTOMATION=true terraform plan"],
	["a shell conditional", "if true; then terraform plan; fi"],
];

for (const [scenario, command] of indirectTerraformCommands) {
	test(`rejects Terraform before the FinOps gate through ${scenario}`, () => {
		const workflow = `
jobs:
  ci-test-gate:
    steps:
      - run: pnpm install --frozen-lockfile
      - run: ${command}
      - run: pnpm check:finops
      - run: terraform -chdir=. validate
${validDeployNeeds}`;

		assert.throws(
			() => validateWorkflowContract(workflow, workflowContracts[1]),
			/Terraform command must not run before pnpm check:finops/u,
		);
	});
}

test("rejects a locked install whose failure is ignored", () => {
	const workflow = `
jobs:
  ci-test-gate:
    steps:
      - run: pnpm install --frozen-lockfile || true
      - run: pnpm check:finops
      - run: terraform -chdir=. validate
${validDeployNeeds}`;

	assert.throws(
		() => validateWorkflowContract(workflow, workflowContracts[1]),
		/locked pnpm install must run before pnpm check:finops/u,
	);
});

test("rejects a deploy workflow whose build job bypasses the gate", () => {
	const workflow = `
jobs:
  ci-test-gate:
    steps:
      - run: pnpm install --frozen-lockfile
      - run: pnpm check:finops
      - run: terraform -chdir=. validate
  build-scan-sign:
    steps: []
  deploy-email:
    needs: [ci-test-gate, build-scan-sign]
    steps: []
`;

	assert.throws(
		() => validateWorkflowContract(workflow, workflowContracts[1]),
		/build-scan-sign must need ci-test-gate/u,
	);
});

test("rejects a build job that runs regardless of gate failure", () => {
	const workflow = `
jobs:
  ci-test-gate:
    steps:
      - run: pnpm install --frozen-lockfile
      - run: pnpm check:finops
      - run: terraform -chdir=. validate
  build-scan-sign:
    needs: ci-test-gate
    if: \${{ always() }}
    steps: []
  deploy-email:
    needs: [ci-test-gate, build-scan-sign]
    if: \${{ success() && github.ref == 'refs/heads/main' }}
    steps: []
`;

	assert.throws(
		() => validateWorkflowContract(workflow, workflowContracts[1]),
		/build-scan-sign must use the required success condition/u,
	);
});

test("rejects a deploy job that runs regardless of dependency failure", () => {
	const workflow = `
jobs:
  ci-test-gate:
    steps:
      - run: pnpm install --frozen-lockfile
      - run: pnpm check:finops
      - run: terraform -chdir=. validate
  build-scan-sign:
    needs: ci-test-gate
    if: \${{ success() }}
    steps: []
  deploy-email:
    needs: [ci-test-gate, build-scan-sign]
    if: \${{ always() }}
    steps: []
`;

	assert.throws(
		() => validateWorkflowContract(workflow, workflowContracts[1]),
		/deploy-email must use the required success and branch condition/u,
	);
});

for (const contract of workflowContracts) {
	test(`${contract.path} enforces FinOps before infrastructure validation`, () => {
		validateWorkflowContract(readFileSync(contract.path, "utf8"), contract);
	});
}
