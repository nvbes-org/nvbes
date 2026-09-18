import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parse } from 'yaml';
import { serviceName, serviceNetwork } from './test-service.mjs';
import {
  candidateGuard,
  fallbackJob,
  fallbackPlan,
  permitsFallbackDispatch,
  runnerSelector,
} from './runner-fallback.core.mjs';

function fixture() {
  const sha = 'a'.repeat(40);
  return {
    repository: 'owner/repo',
    branchSha: sha,
    permission: 'write',
    existing: [],
    run: {
      id: 123,
      path: '.github/workflows/ci.yml',
      event: 'pull_request',
      run_attempt: 1,
      head_repository: { full_name: 'owner/repo' },
      head_sha: sha,
      head_branch: 'codex/candidate',
    },
    jobs: [
      {
        id: 42,
        name: 'authorize-cache',
        conclusion: 'failure',
        steps: [],
        runner_name: '',
        labels: ['ubuntu-latest'],
      },
    ],
    annotations: {
      42: [
        {
          message:
            "The job was not started because recent account payments have failed or your spending limit needs to be increased. Please check the 'Billing & plans' section in your settings",
        },
      ],
    },
    commit: { sha, commit: { verification: { verified: true, reason: 'valid' } } },
  };
}

test('database names isolate runs, attempts and jobs', () => {
  const env = { GITHUB_RUN_ID: '123', GITHUB_RUN_ATTEMPT: '1', GITHUB_JOB: 'database' };
  const name = serviceName('postgres', env);
  for (const change of [
    { GITHUB_RUN_ID: '124' },
    { GITHUB_RUN_ATTEMPT: '2' },
    { GITHUB_JOB: 'rust' },
  ]) {
    assert.notEqual(serviceName('postgres', { ...env, ...change }), name);
  }
  assert.throws(() => serviceName('unsupported', env));
  assert.throws(() => serviceName('postgres', {}));
  assert.throws(() => serviceName('postgres', { ...env, GITHUB_JOB: '../unsafe' }));
});

test('hosted services use ephemeral loopback ports; Docker runners use network DNS', () => {
  assert.deepEqual(serviceNetwork('isolated', 5432), {
    args: ['--publish', '127.0.0.1::5432'],
    host: '127.0.0.1',
  });
  assert.deepEqual(serviceNetwork('isolated', 5432, 'runner-network'), {
    args: ['--network', 'runner-network'],
    host: 'isolated',
    port: 5432,
  });
});

test('confirmed hosted startup failure selects the exact local candidate once', () => {
  assert.deepEqual(fallbackPlan(fixture()), {
    ref: 'codex/candidate',
    inputs: { runner: 'local', expected_sha: 'a'.repeat(40), fallback_of: '123' },
  });
});
for (const [name, mutate] of [
  [
    'test failure',
    (f) => {
      f.jobs[0].steps = [{ name: 'Tests', conclusion: 'failure' }];
    },
  ],
  [
    'allocated runner',
    (f) => {
      f.jobs[0].runner_name = 'hosted-1';
    },
  ],
  [
    'unknown failure',
    (f) => {
      f.annotations[42][0].message = 'Tests failed';
    },
  ],
  [
    'local runner failure',
    (f) => {
      f.jobs[0].labels = ['self-hosted'];
    },
  ],
  [
    'fork',
    (f) => {
      f.run.head_repository.full_name = 'foreign/repo';
    },
  ],
  [
    'stale branch',
    (f) => {
      f.branchSha = 'b'.repeat(40);
    },
  ],
  [
    'unsigned SHA',
    (f) => {
      f.commit.commit.verification.verified = false;
    },
  ],
  [
    'wrong signed commit',
    (f) => {
      f.commit.sha = 'b'.repeat(40);
    },
  ],
  [
    'read-only actor',
    (f) => {
      f.permission = 'read';
    },
  ],
  [
    'duplicate fallback',
    (f) => {
      f.existing = [{ display_title: 'CI local fallback 123' }];
    },
  ],
  [
    'rerun loop',
    (f) => {
      f.run.run_attempt = 2;
    },
  ],
  [
    'other workflow',
    (f) => {
      f.run.path = '.github/workflows/deploy.yml';
    },
  ],
  [
    'missing annotations',
    (f) => {
      f.annotations = {};
    },
  ],
  [
    'no failed jobs',
    (f) => {
      f.jobs = [];
    },
  ],
  [
    'partial execution',
    (f) => {
      f.jobs.push({ ...f.jobs[0], id: 43, name: 'typescript', steps: [{}] });
    },
  ],
]) {
  test(`rejects ${String(name)}`, () => {
    const f = fixture();
    mutate(f);
    assert.throws(() => fallbackPlan(f));
  });
}

for (const file of ['.github/workflows/ci.yml', '.github/workflows/v1-testing.yml']) {
  test(`${file} keeps hosted priority, exact-SHA local routing and a bounded fallback`, () => {
    const text = readFileSync(file, 'utf8');
    const workflow = parse(text);
    assert.equal(workflow.on.workflow_dispatch.inputs.runner.default, 'github-hosted');
    assert.deepEqual(workflow.on.workflow_dispatch.inputs.runner.options, [
      'github-hosted',
      'local',
    ]);
    assert.equal(workflow.on.workflow_run, undefined);
    assert.deepEqual(workflow.jobs['local-fallback'], parse(fallbackJob(file))['local-fallback']);
    assert.equal(permitsFallbackDispatch(file, text), true);
    for (const [name, job] of Object.entries(workflow.jobs)) {
      if (name === 'local-fallback') continue;
      if (file === '.github/workflows/ci.yml' && name === 'security') {
        assert.equal(job['runs-on'], 'ubuntu-latest');
        assert.equal(job.if, undefined);
        continue;
      }
      assert.equal(job['runs-on'], runnerSelector);
      assert(job.if.includes(candidateGuard));
      assert.equal(job.permissions?.actions, undefined);
    }
    assert.equal(
      permitsFallbackDispatch(file, text.replace('actions: read', 'actions: write')),
      false,
    );
    assert.equal(
      permitsFallbackDispatch(
        file,
        text.replace('run: node tools/ci/runner-fallback.mjs', 'run: arbitrary-command'),
      ),
      false,
    );
    assert.equal(
      permitsFallbackDispatch(
        file,
        text.replace(
          "inputs.runner != 'local' && (github.event_name",
          "inputs.runner == 'local' && (github.event_name",
        ),
      ),
      false,
    );
  });
}
