import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';
import { stringify } from 'yaml';
import { validateAccountReleaseReadiness } from './check-account-release-readiness.mjs';
import { validateTestManifest } from './check-test-manifest.mjs';
import { parseWorkflow } from './workflow-contract.mjs';

const scriptRoot = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptRoot, '../..');
const manifest = JSON.parse(
  await readFile(path.join(workspaceRoot, 'docs/testing/account-test-manifest.json'), 'utf8'),
);
const project = JSON.parse(await readFile(path.join(scriptRoot, 'project.json'), 'utf8'));
const workflow = await readFile(
  path.join(workspaceRoot, '.github/workflows/account-quality.yml'),
  'utf8',
);

test('accepts the repository category-to-command-to-artifact contract', () => {
  assert.doesNotThrow(() => validateTestManifest(manifest, project, workflow));
});

test('production readiness refuses gaps, limitations and blocking known gaps', () => {
  assert.throws(
    () => validateAccountReleaseReadiness(manifest),
    /access-control[\s\S]+accessibility[\s\S]+cross-platform[\s\S]+fuzz[\s\S]+load[\s\S]+monkey[\s\S]+reliability[\s\S]+scalability[\s\S]+white-box/u,
  );
  const complete = cloneManifest();
  for (const contract of complete.coverage) {
    if (contract.execution === 'gap') {
      Object.assign(contract, {
        artifact: `${contract.category}.json`,
        command: `pnpm exec nx run ${complete.scope.applications[0]}:test`,
        execution: 'automated',
        lane: 'pullRequest',
      });
      delete contract.blocker;
    }
    delete contract.limitation;
  }
  complete.knownGaps = complete.knownGaps.filter(
    ({ releaseBlocking }) => !releaseBlocking,
  );
  complete.releaseReadiness = {
    blockingCategories: [],
    status: 'ready',
  };
  assert.doesNotThrow(() => validateAccountReleaseReadiness(complete));
});

test('release readiness rejects a stale manual blocker registry', () => {
  const candidate = cloneManifest();
  candidate.releaseReadiness.blockingCategories =
    candidate.releaseReadiness.blockingCategories.filter(
      (category) => category !== 'white-box',
    );
  assert.throws(
    () => validateAccountReleaseReadiness(candidate),
    /blocker registry is stale/u,
  );
  assert.throws(
    () => validateTestManifest(candidate, project, workflow),
    /blockingCategories must exactly match/u,
  );
});

test('rejects an automated fuzz claim', () => {
  const candidate = cloneManifest();
  const fuzz = candidate.coverage.find(({ category }) => category === 'fuzz');
  Object.assign(fuzz, {
    artifact: 'fuzz.json',
    command: 'cargo fuzz run parser',
    execution: 'automated',
    lane: 'pullRequest',
  });
  delete fuzz.blocker;
  assert.throws(() => validateTestManifest(candidate, project, workflow), /fuzz/u);
});

test('rejects a scheduled category moved to a non-scheduled lane', () => {
  const candidate = cloneManifest();
  candidate.coverage.find(({ category }) => category === 'cookies').lane = 'pullRequest';
  assert.throws(() => validateTestManifest(candidate, project, workflow), /cookies/u);
});

test('rejects missing artifacts and cron drift', () => {
  const withoutArtifact = cloneManifest();
  delete withoutArtifact.coverage.find(({ category }) => category === 'load').artifact;
  assert.throws(() => validateTestManifest(withoutArtifact, project, workflow), /artifact/u);

  const cronDrift = cloneManifest();
  cronDrift.automation.scheduledDailyCron = '0 0 * * *';
  assert.throws(() => validateTestManifest(cronDrift, project, workflow), /workflow crons/u);
});

test('rejects scalability advertised without control-plane attestation', () => {
  const candidate = cloneManifest();
  const scalability = candidate.coverage.find(({ category }) => category === 'scalability');
  Object.assign(scalability, {
    artifact: '/evidence/scalability-verdict.json',
    command:
      'ACCOUNT_SCALABILITY_COMPARISON_FILE=/evidence/comparison.json K6_PROFILE=scalability pnpm nx run account-quality:test-resilience',
    execution: 'manual-command',
    lane: 'preRelease',
  });
  delete scalability.blocker;
  assert.throws(() => validateTestManifest(candidate, project, workflow), /explicit gap/u);
});

test('rejects a workflow without fail-closed scheduling or required artifacts', () => {
  const withoutGate = mutateWorkflow((candidate) => {
    const step = namedStep(
      candidate.jobs['campaign-gate'],
      'Fail closed when a scheduled campaign is disabled',
    );
    step.run = 'echo disabled';
  });
  assert.throws(() => validateTestManifest(manifest, project, withoutGate), /fail closed/u);

  const warningArtifact = mutateWorkflow((candidate) => {
    const upload = candidate.jobs['nightly-load'].steps.find((step) =>
      step.uses?.startsWith('actions/upload-artifact@'),
    );
    upload.with['if-no-files-found'] = 'warn';
  });
  assert.throws(
    () => validateTestManifest(manifest, project, warningArtifact),
    /nightly-load evidence upload/u,
  );

  const permissiveGate = mutateWorkflow((candidate) => {
    const step = namedStep(
      candidate.jobs['campaign-gate'],
      'Fail closed when a scheduled campaign is disabled',
    );
    step.run = step.run.replace(
      '"$ACCOUNT_TEST_AUTOMATION_ENABLED" != "true"',
      '"$ACCOUNT_TEST_AUTOMATION_ENABLED" == "true"',
    );
  });
  assert.throws(
    () => validateTestManifest(manifest, project, permissiveGate),
    /fail closed unless scheduling is explicitly enabled/u,
  );
});

test('rejects an inexact manual duration contract', () => {
  const candidate = mutateWorkflow((parsed) => {
    parsed.on.workflow_dispatch.inputs.duration.options = [
      'profile-default',
      '6m',
      '10m',
      '30m',
      '4h',
    ];
  });
  assert.throws(
    () => validateTestManifest(manifest, project, candidate),
    /workflow input duration/u,
  );
});

test('rejects a deleted workflow job and a partial weekly profile matrix', () => {
  const withoutLoadJob = mutateWorkflow((candidate) => {
    delete candidate.jobs['nightly-load'];
  });
  assert.throws(
    () => validateTestManifest(manifest, project, withoutLoadJob),
    /workflow jobs keys/u,
  );

  const withoutSpike = mutateWorkflow((candidate) => {
    candidate.jobs['weekly-resilience'].strategy.matrix.profile = ['volume', 'stress'];
  });
  assert.throws(
    () => validateTestManifest(manifest, project, withoutSpike),
    /weekly resilience matrix/u,
  );
});

test('rejects an unexpected dispatch input', () => {
  const candidate = mutateWorkflow((parsed) => {
    parsed.on.workflow_dispatch.inputs.unsafe_target = {
      description: 'Uncontracted target',
      required: false,
      type: 'string',
    };
  });
  assert.throws(
    () => validateTestManifest(manifest, project, candidate),
    /workflow_dispatch inputs keys/u,
  );
});

test('rejects Nx cache inputs that omit the load suite', () => {
  const candidateProject = structuredClone(project);
  candidateProject.targets.test.inputs = candidateProject.targets.test.inputs.filter(
    (input) => input !== '{workspaceRoot}/tools/load-tests/account/**/*',
  );
  assert.throws(
    () => validateTestManifest(manifest, candidateProject, workflow),
    /inputs must cover/u,
  );
});

function cloneManifest() {
  return structuredClone(manifest);
}

function mutateWorkflow(mutation) {
  const candidate = parseWorkflow(workflow);
  mutation(candidate);
  return stringify(candidate);
}

function namedStep(job, name) {
  return job.steps.find((step) => step.name === name);
}
