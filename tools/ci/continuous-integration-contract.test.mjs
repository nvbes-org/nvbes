import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parse, stringify } from 'yaml';
import { validateContinuousWorkflow } from './continuous-workflow.core.mjs';

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
