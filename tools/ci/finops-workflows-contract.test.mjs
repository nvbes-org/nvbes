import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parse } from 'yaml';

const workflowContracts = [
  { path: '.github/workflows/ci.yml', job: 'email-quality' },
  {
    path: '.github/workflows/deploy-email.yml',
    job: 'ci-test-gate',
    buildJob: 'build-scan-sign',
    deployJob: 'deploy-email',
  },
  {
    path: '.github/workflows/deploy-trust-risk.yml',
    job: 'ci-test-gate',
    buildJob: 'build-scan-sign',
    deployJob: 'deploy-trust-risk',
  },
];

const validDeployNeeds = `
  build-scan-sign:
    needs: ci-test-gate
    steps: []
  deploy-email:
    needs: [ci-test-gate, build-scan-sign]
    steps: []
`;

test('rejects an invocation moved outside the gate even when a gate comment mentions it', () => {
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

test('rejects an inline Terraform command before the FinOps gate', () => {
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
  ['a chained directory change', 'cd infrastructure && terraform plan'],
  ['the command builtin', 'command terraform plan'],
  ['the env utility', 'env TF_IN_AUTOMATION=true terraform plan'],
  ['a shell conditional', 'if true; then terraform plan; fi'],
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

test('rejects a locked install whose failure is ignored', () => {
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

test('rejects a deploy workflow whose build job bypasses the gate', () => {
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

function isRecord(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function commandsForJob(job) {
  assert.ok(isRecord(job), 'workflow job must be an object');
  assert.ok(Array.isArray(job.steps), 'workflow job must contain steps');

  return job.steps.flatMap((step, stepIndex) => {
    if (!isRecord(step) || typeof step.run !== 'string') return [];

    return step.run
      .split(/\r?\n/u)
      .map((line, lineIndex) => ({ command: line.trim(), lineIndex, stepIndex }))
      .filter(({ command }) => command.length > 0 && !command.startsWith('#'));
  });
}

function isFinOpsCommand(command) {
  return command === 'pnpm check:finops';
}

function isLockedInstall(command) {
  return (
    command === 'pnpm install --frozen-lockfile' ||
    command === 'pnpm install --frozen-lockfile --prefer-offline'
  );
}

function isTerraformCommand(command) {
  return /(^|[^A-Za-z0-9_])terraform(?=$|[^A-Za-z0-9_])/u.test(command);
}

function jobNeeds(job) {
  if (!isRecord(job)) return [];
  if (typeof job.needs === 'string') return [job.needs];
  return Array.isArray(job.needs) ? job.needs : [];
}

function validateWorkflowContract(source, contract) {
  const workflow = parse(source);
  assert.ok(isRecord(workflow) && isRecord(workflow.jobs), 'workflow must define jobs');

  const gate = workflow.jobs[contract.job];
  const gateCommands = commandsForJob(gate);
  const gateFinOpsIndexes = gateCommands
    .map(({ command }, index) => (isFinOpsCommand(command) ? index : -1))
    .filter((index) => index >= 0);

  assert.equal(
    gateFinOpsIndexes.length,
    1,
    `${contract.job} must invoke pnpm check:finops exactly once`,
  );

  for (const [jobName, job] of Object.entries(workflow.jobs)) {
    if (jobName === contract.job) continue;
    assert.equal(
      commandsForJob(job).filter(({ command }) => isFinOpsCommand(command)).length,
      0,
      `${jobName} must not invoke pnpm check:finops`,
    );
  }

  const finOpsIndex = gateFinOpsIndexes[0];
  assert.ok(
    gateCommands.slice(0, finOpsIndex).some(({ command }) => isLockedInstall(command)),
    'locked pnpm install must run before pnpm check:finops',
  );
  assert.equal(
    gateCommands.slice(0, finOpsIndex).some(({ command }) => isTerraformCommand(command)),
    false,
    'Terraform command must not run before pnpm check:finops',
  );
  assert.ok(
    gateCommands.slice(finOpsIndex + 1).some(({ command }) => isTerraformCommand(command)),
    'Terraform validation must run after pnpm check:finops',
  );

  if (contract.buildJob === undefined || contract.deployJob === undefined) return;
  assert.ok(
    jobNeeds(workflow.jobs[contract.buildJob]).includes(contract.job),
    `${contract.buildJob} must need ${contract.job}`,
  );
  const deployNeeds = jobNeeds(workflow.jobs[contract.deployJob]);
  assert.ok(deployNeeds.includes(contract.job), `${contract.deployJob} must need ${contract.job}`);
  assert.ok(
    deployNeeds.includes(contract.buildJob),
    `${contract.deployJob} must need ${contract.buildJob}`,
  );
}

for (const contract of workflowContracts) {
  test(`${contract.path} enforces FinOps before infrastructure validation`, () => {
    validateWorkflowContract(readFileSync(contract.path, 'utf8'), contract);
  });
}
