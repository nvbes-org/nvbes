import assert from 'node:assert/strict';
import test from 'node:test';
import { resolveScopeBase } from './scope-base.mjs';
import { classifyPaths } from './scope-paths.mjs';
import { createPlan, lanes } from './scope-plan.mjs';
import { gateFailures } from './ci-gate.core.mjs';
import { loadTerraformSources, terraformProjects } from './scope-terraform.mjs';

const node = (root, targets) => ({
  data: { root, targets: Object.fromEntries(targets.map((t) => [t, {}])) },
});
const nodes = {
  consumer: node('libs/ts/consumer', ['check']),
  tests: node('libs/ts/tests', ['test']),
  'account-service': node('apps/account-service', ['test:database', 'test:contract']),
  'email-worker': node('apps/email-worker', ['test:database', 'test:contract']),
  tfAccount: node('infrastructure/environments/account', ['terraform:validate']),
  tfEmail: node('infrastructure/environments/email', ['terraform:validate']),
};
function plan(paths, options = {}) {
  return createPlan(
    {
      paths,
      ...classifyPaths(paths),
      fallback: false,
      base: 'a'.repeat(40),
      head: 'b'.repeat(40),
      ...options,
    },
    {
      nodes,
      affected: ['consumer', 'tests'],
      read: () => 'COPY apps ./apps\nCOPY libs ./libs\n',
      ...options.context,
    },
  );
}

test('documentation fast path needs no runtime or graph', () => {
  const p = createPlan({ paths: ['docs/readme.md'], ...classifyPaths(['docs/readme.md']) });
  assert.deepEqual(
    lanes.filter((l) => p.candidate[l]),
    [],
  );
  assert.equal(classifyPaths(['docs/security/policy.json']).graphRequired, true);
});
test('Actions-only fast path runs CI contracts', () => {
  const paths = ['.github/workflows/ci.yml'];
  const p = createPlan({ paths, ...classifyPaths(paths) });
  assert.deepEqual(
    lanes.filter((l) => p.candidate[l]),
    ['contracts'],
  );
});
test('Email Dockerfile never selects Rust or a different container', () => {
  const p = plan(['apps/email-worker/Dockerfile']);
  assert.equal(p.candidate.rust, false);
  assert.deepEqual(p.containers, ['email-worker']);
  assert.deepEqual(p.databases, []);
});
test('selects available check and test targets independently', () => {
  const p = plan(['libs/ts/consumer/src.ts']);
  assert.deepEqual(p.targets.check, ['consumer']);
  assert.deepEqual(p.targets.test, ['tests']);
  assert.deepEqual(p.targets.lint, []);
});
test('npm manifest dependency updates do not trigger unrelated runtimes', () => {
  const p = plan(['package.json', 'pnpm-lock.yaml']);
  assert.equal(p.candidate.rust, false);
  assert.equal(p.candidate.database, false);
  assert.equal(p.candidate.terraform, false);
  assert.equal(p.candidate.typescript, true);
});
test('source copied into containers selects the consuming contracts', () => {
  assert.deepEqual(plan(['libs/rust/core/src/lib.rs']).containers, [
    'account-service',
    'email-worker',
  ]);
});
test('generic application logic does not select database tests', () => {
  assert.deepEqual(plan(['apps/account-service/src/account.service.rs']).databases, []);
});
test('SQL in a business-named file still selects its database scope', () => {
  assert.deepEqual(
    plan(['apps/account-service/src/account.teams.rs'], {
      context: { read: () => 'sqlx::query("SELECT 1")' },
    }).databases,
    ['account-service'],
  );
});
test('Account migrations select Account DB without Rust', () => {
  const p = plan(['apps/account-service/migrations/002.sql']);
  assert.deepEqual(p.databases, ['account-service']);
  assert.equal(p.candidate.rust, false);
});
test('shared persistence and SQL dependency changes select DB consumers', () => {
  assert.deepEqual(plan(['libs/rust/core/src/core.db.rs']).databases, [
    'account-service',
    'email-worker',
  ]);
  assert.equal(plan(['Cargo.lock']).candidate.rust, true);
});
test('unknown root and unavailable base select the complete scope', () => {
  for (const p of [plan(['unknown.config']), plan([], { fallback: true })]) {
    assert.equal(p.fallbackFull, true);
    assert.ok(lanes.every((lane) => p.candidate[lane]));
  }
});
test('Nx errors fall back to all projects', () => {
  const p = plan(['pnpm-lock.yaml'], { context: { graphFallback: true } });
  assert.equal(p.fallbackFull, true);
  assert.deepEqual(p.targets.check, ['consumer']);
});
test('Terraform uses local module consumers and falls back on unknown infra', async () => {
  const files = [
    'infrastructure/environments/account/main.tf',
    'infrastructure/environments/email/main.tf',
  ];
  const read = (file) => [
    file.includes('account')
      ? 'module "infra" { source = "../../modules/shared" }'
      : 'module "infra" { source = "../../modules/email" }',
  ];
  const sources = await loadTerraformSources(files, read);
  assert.deepEqual(terraformProjects(['infrastructure/modules/shared/main.tf'], nodes, sources), [
    'tfAccount',
  ]);
  assert.deepEqual(
    terraformProjects(['infrastructure/environments/email/.terraform.lock.hcl'], nodes, sources),
    ['tfEmail'],
  );
  assert.deepEqual(terraformProjects(['infrastructure/unknown.tf'], nodes, sources), [
    'tfAccount',
    'tfEmail',
  ]);
  assert.deepEqual(terraformProjects(['infrastructure/modules/shared/main.tf'], nodes, null), [
    'tfAccount',
    'tfEmail',
  ]);
});

const sha = (character) => character.repeat(40);
const environment = {
  event: {},
  head: sha('c'),
  branch: 'dev',
  repository: 'org/repo',
  runId: '20',
};
const successful = {
  id: 1,
  head_sha: sha('a'),
  head_branch: 'dev',
  event: 'push',
  conclusion: 'success',
  path: '.github/workflows/ci.yml',
  repository: { full_name: 'org/repo' },
};
const io = {
  exists: () => true,
  ancestor: () => true,
  runs: async function* () {
    yield successful;
  },
};
test('push after failure compares against the earlier successful CI', async () => {
  const base = await resolveScopeBase(environment, {
    ...io,
    runs: async function* () {
      yield { ...successful, conclusion: 'failure', head_sha: sha('b') };
      yield successful;
    },
  });
  assert.equal(base.base, sha('a'));
});
test('PR uses its explicit base even without API access', async () => {
  const base = await resolveScopeBase(
    { ...environment, event: { pull_request: { base: { sha: sha('b') } } } },
    io,
  );
  assert.equal(base.base, sha('b'));
});
test('missing base, inaccessible history, and missing ancestor use full fallback', async () => {
  assert.equal(
    (await resolveScopeBase({ ...environment, explicitBase: 'bad' }, io)).fallback,
    true,
  );
  assert.equal(
    (await resolveScopeBase(environment, { ...io, ancestor: () => false })).fallback,
    true,
  );
  assert.equal(
    (
      await resolveScopeBase(environment, {
        ...io,
        runs: async function* () {
          throw new Error('403');
        },
      })
    ).fallback,
    true,
  );
});
test('dispatch accepts a validated explicit base', async () => {
  assert.equal(
    (await resolveScopeBase({ ...environment, explicitBase: sha('b') }, io)).base,
    sha('b'),
  );
});
test('gate rejects failures, cancelled, missing and unexpectedly skipped lanes', () => {
  const p = plan(['unknown.config']);
  const needs = Object.fromEntries(
    ['scope', 'authorize-cache', ...lanes].map((lane) => [lane, { result: 'success' }]),
  );
  needs.scope.outputs = { plan: JSON.stringify(p) };
  assert.deepEqual(gateFailures(needs), []);
  for (const result of ['failure', 'cancelled', 'skipped', undefined]) {
    assert.ok(gateFailures({ ...needs, rust: { result } }).length > 0);
  }
  assert.ok(gateFailures({ ...needs, scope: { result: 'success' } }).length > 0);
  p.candidate.rust = false;
  needs.scope.outputs.plan = JSON.stringify(p);
  assert.deepEqual(gateFailures({ ...needs, rust: { result: 'skipped' } }), []);
});
