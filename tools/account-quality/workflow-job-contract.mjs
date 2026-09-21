import { isDeepStrictEqual } from 'node:util';

const dailyCondition =
  "(github.event_name == 'schedule' && github.event.schedule == '17 1 * * *') || " +
  "(github.event_name == 'workflow_dispatch' && inputs.lane == 'nightly')";
const weeklyCondition = "github.event_name == 'schedule' && github.event.schedule == '31 1 * * 0'";
const githubRunId = `\${{ github.run_id }}`;
const inputProfile = `\${{ inputs.profile }}`;
const matrixProfile = `\${{ matrix.profile }}`;
const checkout = 'uses:actions/checkout';
const securityGate = 'name:CI/CD security gate';
const setupNode = 'uses:actions/setup-node';

const jobContract = {
  'browser-compatibility': {
    artifact: {
      name: `account-browser-${githubRunId}`,
      path: 'apps/account-web/playwright-report/\napps/account-web/test-results/\n',
    },
    condition: dailyCondition,
    command: 'pnpm nx run account-web:test:e2e:compatibility',
    commandStep: 'Run accessibility, cookie, UI and compatibility tests',
    steps: [
      checkout,
      securityGate,
      'name:Enable pnpm',
      setupNode,
      'name:Install dependencies',
      'name:Install browser compatibility matrix',
      'name:Run accessibility, cookie, UI and compatibility tests',
      'name:Upload required browser evidence',
    ],
  },
  'manual-resilience': {
    artifact: {
      name: `account-${inputProfile}-${githubRunId}`,
      path: '.temp/account-quality/k6/',
    },
    condition: "github.event_name == 'workflow_dispatch' && inputs.lane == 'resilience'",
    command: 'bash tools/account-quality/run-account-resilience.sh',
    commandStep: 'Run exact manual resilience profile',
    steps: [
      checkout,
      securityGate,
      'name:Run exact manual resilience profile',
      'name:Upload required manual resilience evidence',
    ],
  },
  'nightly-load': {
    artifact: {
      name: `account-load-${githubRunId}`,
      path: '.temp/account-quality/k6/',
    },
    condition: dailyCondition,
    command: 'bash tools/account-quality/run-account-k6.sh',
    commandStep: 'Run Account nominal load profile',
    steps: [
      checkout,
      securityGate,
      'name:Run Account nominal load profile',
      'name:Upload required load evidence',
    ],
  },
  'weekly-resilience': {
    artifact: {
      name: `account-${matrixProfile}-${githubRunId}`,
      path: '.temp/account-quality/k6/',
    },
    condition: weeklyCondition,
    command: 'bash tools/account-quality/run-account-resilience.sh',
    commandStep: 'Run serialized resilience profile',
    steps: [
      checkout,
      securityGate,
      'name:Run serialized resilience profile',
      'name:Upload required resilience evidence',
    ],
  },
};

export function validateWorkflowJobs(jobs) {
  assert(isRecord(jobs), 'workflow jobs are missing');
  assertExactKeys(
    jobs,
    [
      'browser-compatibility',
      'campaign-gate',
      'manual-resilience',
      'nightly-load',
      'quality-contract',
      'weekly-resilience',
    ],
    'workflow jobs',
  );
  validateContractJob(jobs['quality-contract']);
  validateCampaignGate(jobs['campaign-gate']);
  for (const [jobName, contract] of Object.entries(jobContract)) {
    validateCampaignJob(jobName, jobs[jobName], contract);
  }
  validateWeeklyMatrix(jobs['weekly-resilience']);
  assert(jobs['nightly-load'].env?.K6_PROFILE === 'load', 'nightly load must use profile load');

  const uploadCount = Object.values(jobs)
    .flatMap((job) => job.steps ?? [])
    .filter((step) => isUploadStep(step)).length;
  assert(uploadCount === 4, 'workflow must contain exactly four evidence upload steps');
}

function validateContractJob(job) {
  assert(isRecord(job), 'quality-contract job is missing');
  assertStepSequence(job, [
    checkout,
    securityGate,
    'name:Enable pnpm',
    setupNode,
    'name:Install dependencies',
    'name:Validate Account quality contracts',
  ]);
  requireRunStep(job, 'CI/CD security gate', 'node tools/security/check-ci-cd-security.mjs');
  requireRunStep(
    job,
    'Validate Account quality contracts',
    'pnpm exec nx run account-quality:test',
  );
}

function validateCampaignGate(job) {
  assert(isRecord(job), 'campaign-gate job is missing');
  assertStepSequence(job, [
    checkout,
    'name:Fail closed when a scheduled campaign is disabled',
    'name:Validate lane, exact targets and duration',
  ]);
  const failClosed = requireNamedStep(job, 'Fail closed when a scheduled campaign is disabled');
  assert(
    failClosed.run?.includes('"$ACCOUNT_TEST_AUTOMATION_ENABLED" != "true"'),
    'campaign-gate must fail closed unless scheduling is explicitly enabled',
  );
  const validation = requireNamedStep(job, 'Validate lane, exact targets and duration');
  for (const command of [
    'validate-load-target.mjs web',
    'validate-load-target.mjs service',
    'validate-resilience-inputs.mjs',
  ]) {
    assert(validation.run?.includes(command), `campaign-gate is missing ${command}`);
  }
}

function validateCampaignJob(jobName, job, contract) {
  assert(isRecord(job), `${jobName} job is missing`);
  assertStepSequence(job, contract.steps);
  assertDeepEqual(job.needs, ['campaign-gate', 'quality-contract'], `${jobName} dependencies`);
  assert(job.if === contract.condition, `${jobName} trigger condition differs from its contract`);
  requireRunStep(job, 'CI/CD security gate', 'node tools/security/check-ci-cd-security.mjs');
  requireRunStep(job, contract.commandStep, contract.command);
  validateArtifact(jobName, job, contract.artifact);
}

function validateArtifact(jobName, job, artifact) {
  const upload = (job.steps ?? []).filter((step) => isUploadStep(step));
  assert(upload.length === 1, `${jobName} must contain exactly one evidence upload`);
  assert(upload[0].if === 'always()', `${jobName} evidence must upload even after failure`);
  assertDeepEqual(
    upload[0].with,
    {
      'if-no-files-found': 'error',
      name: artifact.name,
      path: artifact.path,
      'retention-days': 30,
    },
    `${jobName} evidence upload`,
  );
}

function validateWeeklyMatrix(job) {
  assert(job.strategy?.['fail-fast'] === false, 'weekly matrix must collect every profile result');
  assert(job.strategy?.['max-parallel'] === 1, 'weekly matrix must be serialized');
  assertDeepEqual(
    job.strategy?.matrix,
    { profile: ['volume', 'spike', 'stress'] },
    'weekly resilience matrix',
  );
  assert(
    job.env?.K6_PROFILE === matrixProfile,
    'weekly profile must come exclusively from matrix.profile',
  );
}

function requireRunStep(job, name, run) {
  const step = requireNamedStep(job, name);
  assert(step.run === run, `${name} must run exactly ${run}`);
  assert(!step.uses, `${name} cannot delegate to an action`);
}

function requireNamedStep(job, name) {
  assert(Array.isArray(job.steps), 'job steps must be an array');
  const matches = job.steps.filter((step) => step.name === name);
  assert(matches.length === 1, `workflow must contain exactly one ${name} step`);
  return matches[0];
}

function assertStepSequence(job, expected) {
  assertDeepEqual(
    job.steps?.map((step) =>
      step.name ? `name:${step.name}` : `uses:${step.uses?.split('@')[0]}`,
    ),
    expected,
    'workflow job steps',
  );
}

function isUploadStep(step) {
  return typeof step?.uses === 'string' && step.uses.startsWith('actions/upload-artifact@');
}

function assertExactKeys(record, expected, label) {
  assertDeepEqual(
    Object.keys(record).sort((a, b) => a.localeCompare(b)),
    [...expected].sort((a, b) => a.localeCompare(b)),
    `${label} keys`,
  );
}

function assertDeepEqual(actual, expected, label) {
  assert(isDeepStrictEqual(actual, expected), `${label} differs from its exact contract`);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function isRecord(value) {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
