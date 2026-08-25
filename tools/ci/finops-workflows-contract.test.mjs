import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const workflowContracts = [
  { path: '.github/workflows/ci.yml', job: 'email-quality' },
  { path: '.github/workflows/deploy-email.yml', job: 'ci-test-gate' },
  { path: '.github/workflows/deploy-trust-risk.yml', job: 'ci-test-gate' },
];

function jobBody(workflow, jobName) {
  const startMarker = `  ${jobName}:\n`;
  const start = workflow.indexOf(startMarker);
  assert.notEqual(start, -1, `missing ${jobName} job`);

  const remaining = workflow.slice(start + startMarker.length);
  const nextJob = remaining.match(/^  [a-z0-9-]+:\s*$/mu);
  const end = nextJob?.index === undefined ? undefined : start + startMarker.length + nextJob.index;
  return workflow.slice(start, end);
}

for (const { path, job } of workflowContracts) {
  test(`${path} enforces FinOps in ${job} before infrastructure validation`, () => {
    const workflow = readFileSync(path, 'utf8');
    const gate = jobBody(workflow, job);
    const invocations = workflow.match(/^\s*pnpm check:finops\s*$/gmu) ?? [];
    const installIndex = gate.indexOf('pnpm install --frozen-lockfile');
    const finopsIndex = gate.indexOf('pnpm check:finops');
    const terraformIndex = gate.search(/^\s*terraform -chdir=/mu);

    assert.equal(invocations.length, 1, 'workflow must invoke the FinOps gate exactly once');
    assert.notEqual(installIndex, -1, 'locked pnpm install must exist in the gate job');
    assert.notEqual(finopsIndex, -1, 'FinOps gate must run in the gate job');
    assert.notEqual(terraformIndex, -1, 'Terraform validation must exist in the gate job');
    assert.ok(installIndex < finopsIndex, 'FinOps gate must run after dependency installation');
    assert.ok(finopsIndex < terraformIndex, 'FinOps gate must run before Terraform validation');
  });
}
