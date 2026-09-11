import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parse, stringify } from 'yaml';
import { validateContinuousWorkflow } from './continuous-workflow.core.mjs';
import { spawnSync } from 'node:child_process';
import { localGuardCommand, validateRunnerFallback } from './runner-fallback-contract.mjs';

const workflow = readFileSync('.github/workflows/ci.yml', 'utf8');
const setup = readFileSync('.github/actions/ci-setup/action.yml', 'utf8');
test('CI preserves runtime isolation, FinOps, and the required gate', () => {
  validateContinuousWorkflow(workflow, setup);
  const parsed = parse(workflow);
  assert.deepEqual(parsed.on.push.branches, ['main']);
  assert.deepEqual(parsed.on.pull_request.branches, ['main']);
  assert.equal(parsed.on.pull_request_target, undefined);
  assert.equal(parsed.concurrency['cancel-in-progress'], true);
  assert.equal(parsed.env.NX_DAEMON, 'false');
  assert.equal(parsed.env.NX_WORKSPACE_DATA_DIRECTORY, '.nx/cache/workspace-data');
});
for (const [name, mutate] of [
  ['missing gate dependency', (w) => w.jobs['ci-gate'].needs.pop()],
  ['Terraform bypasses FinOps', (w) => w.jobs.terraform.needs.pop()],
  [
    'conditional FinOps',
    (w) => {
      w.jobs.scope.steps.find((s) => s.run === 'pnpm check:finops').if = 'false';
    },
  ],
  [
    'disguised FinOps',
    (w) => {
      w.jobs.scope.steps.find((s) => s.run === 'pnpm check:finops').run =
        'exit 0\npnpm check:finops';
    },
  ],
  [
    'Rust installed for TypeScript',
    (w) => {
      w.jobs.typescript.steps.find((s) => s.with?.caches).with.rust = 'true';
    },
  ],
  [
    'ignored Rust failure',
    (w) => {
      w.jobs.rust['continue-on-error'] = true;
    },
  ],
  [
    'environment injection',
    (w) => {
      w.jobs.terraform.env.BASH_ENV = '/tmp/inject';
    },
  ],
  [
    'unconditional graph setup',
    (w) => {
      delete w.jobs.scope.steps.find((s) => s.uses === './.github/actions/ci-setup').if;
    },
  ],
]) {
  test(`rejects ${String(name)}`, () => {
    const changed = parse(workflow);
    mutate(changed);
    assert.throws(() => validateContinuousWorkflow(stringify(changed), setup));
  });
}

for (const file of ['ci.yml', 'v1-testing.yml']) {
  const original = parse(readFileSync(`.github/workflows/${file}`, 'utf8'));
  test(`${file} offers manual local fallback for every job with hosted default`, () => {
    validateRunnerFallback(original);
  });
  for (const [label, mutate] of [
    [
      'automatic local default',
      (w) => {
        w.on.workflow_dispatch.inputs.runner.default = 'local';
      },
    ],
    [
      'arbitrary runner labels',
      (w) => {
        w.on.workflow_dispatch.inputs.runner.type = 'string';
      },
    ],
    [
      'stranded hosted job',
      (w) => {
        Object.values(w.jobs)[0]['runs-on'] = 'ubuntu-latest';
      },
    ],
    [
      'unconditional self-hosted job',
      (w) => {
        Object.values(w.jobs)[0]['runs-on'] = ['self-hosted'];
      },
    ],
    [
      'missing platform guard',
      (w) => {
        Object.values(w.jobs)[0].steps.shift();
      },
    ],
    [
      'ignored platform failure',
      (w) => {
        Object.values(w.jobs)[0].steps[0]['continue-on-error'] = true;
      },
    ],
    [
      'candidate override',
      (w) => {
        Object.values(w.jobs)[0].env = { NVBES_CI_EXPECTED_SHA: 'other' };
      },
    ],
  ]) {
    assert.equal(typeof label, 'string');
    test(`${file} rejects ${String(label)}`, () => {
      const changed = structuredClone(original);
      mutate(changed);
      assert.throws(() => validateRunnerFallback(changed));
    });
  }
}

for (const [label, overrides, succeeds] of [
  ['matching Linux candidate', {}, true],
  ['wrong commit', { NVBES_CI_EXPECTED_SHA: 'b'.repeat(40) }, false],
  ['missing candidate', { NVBES_CI_EXPECTED_SHA: '' }, false],
  ['short candidate', { NVBES_CI_EXPECTED_SHA: 'a'.repeat(7) }, false],
  ['mislabeled runner', { RUNNER_OS: 'macOS' }, false],
  ['wrong kernel', { FIXTURE_KERNEL: 'Darwin' }, false],
  ['non-Linux Docker', { FIXTURE_DOCKER: 'windows' }, false],
]) {
  if (typeof label !== 'string') throw new Error('Invalid fixture label');
  test(`local runner guard handles ${label}`, () => {
    const result = spawnSync(
      'bash',
      [
        '--noprofile',
        '--norc',
        '-euo',
        'pipefail',
        '-c',
        `uname() { printf '%s' "$FIXTURE_KERNEL"; }; docker() { printf '%s' "$FIXTURE_DOCKER"; };\n${localGuardCommand}`,
      ],
      {
        env: {
          PATH: process.env.PATH,
          RUNNER_OS: 'Linux',
          FIXTURE_KERNEL: 'Linux',
          FIXTURE_DOCKER: 'linux',
          GITHUB_SHA: 'a'.repeat(40),
          NVBES_CI_EXPECTED_SHA: 'a'.repeat(40),
          ...overrides,
        },
      },
    );
    assert.equal(result.status === 0, succeeds);
  });
}
