import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parse, stringify } from 'yaml';
import { validateContinuousWorkflow } from './continuous-workflow.core.mjs';

const workflow = readFileSync('.github/workflows/ci.yml', 'utf8');
const setup = readFileSync('.github/actions/ci-setup/action.yml', 'utf8');
test('Rust reports use Node 24 uploads and explicitly include only their hidden report paths', () => {
  const steps = parse(workflow).jobs.rust.steps;
  const uploads = steps.filter((step) => step.uses?.startsWith('actions/upload-artifact@'));
  assert.equal(uploads.length, 2);
  for (const step of uploads) {
    assert.equal(step.uses, 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a');
    assert.equal(step.with['include-hidden-files'], true);
    assert.equal(step.with['if-no-files-found'], 'error');
    assert.match(
      step.with.path,
      /^\.temp\/(rust\/coverage-workspace|v1-receipts\/platform\.workspace-check)\.json$/u,
    );
  }
  assert.equal(steps.find((step) => step.name === 'Validate rust coverage').id, 'rust-coverage');
  assert.equal(
    steps.find((step) => step.name === 'Upload rust coverage report').if,
    "${{ !cancelled() && steps.rust-coverage.outcome != 'skipped' }}",
  );
  assert.equal(
    parse(setup).runs.steps.find((step) => step.uses?.startsWith('actions/setup-node@')).with[
      'node-version'
    ],
    24,
  );
});

test('PostgreSQL limiter tests remain components, without stale Redis classifications', () => {
  const components = JSON.parse(
    readFileSync('tools/rust-workspace/micro-test.components.json', 'utf8'),
  );
  assert.deepEqual(components['nvbes-core'], {
    'limiter::tests::rate_limiter_blocks_after_limit': 'PostgreSQL persistence',
    'limiter::postgres_tests::': 'PostgreSQL persistence and concurrency',
    'postgres_runtime::integration_tests::':
      'PostgreSQL pool integration requires NVBES_SECURITY_TEST_DATABASE_URL',
  });
  assert.equal(components['nvbes-redis'], undefined);
});
test('CI preserves runtime isolation, FinOps, and the required gate', () => {
  validateContinuousWorkflow(workflow, setup);
  const parsed = parse(workflow);
  assert.deepEqual(parsed.on.push.branches, ['main']);
  assert.deepEqual(parsed.on.pull_request.branches, ['main']);
  assert.ok(parsed.on.merge_group !== undefined);
  assert.deepEqual(Object.keys(parsed.on.workflow_dispatch.inputs).sort(), ['base_sha', 'mode']);
  assert.equal(parsed.on.pull_request_target, undefined);
  assert.equal(
    parsed.concurrency['cancel-in-progress'],
    "${{ github.event_name != 'merge_group' }}",
  );
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
    'missing micro-test Node runtime',
    (w) => {
      w.jobs.rust.steps.find((s) => s.with?.caches).with.node = 'false';
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
