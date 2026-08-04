import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import { validateEmailTestDatabaseTarget } from './validate-email-test-database.mjs';

const root = new URL('../', import.meta.url);

async function workspaceFile(path) {
  return readFile(new URL(path, root), 'utf8');
}

test('development entrypoints start the identity email queue consumer', async () => {
  const [devApi, devAccount] = await Promise.all([
    workspaceFile('scripts/dev-api.sh'),
    workspaceFile('scripts/dev-account.sh'),
  ]);

  for (const script of [devApi, devAccount]) {
    assert.match(script, /dev-identity-worker\.sh/);
  }
});

test('identity worker has an explicit development command', async () => {
  const packageJson = JSON.parse(await workspaceFile('package.json'));
  assert.equal(packageJson.scripts['dev:identity-worker'], 'bash scripts/dev-identity-worker.sh');
  assert.match(packageJson.scripts.check, /pnpm check:dev-runtime/);

  const launcher = await workspaceFile('scripts/dev-identity-worker.sh');
  assert.match(launcher, /cargo run -p nvbes-identity-worker/);
  assert.match(launcher, /export NVBES_DATABASE_URL="\$\{NVBES_IDENTITY_DATABASE_URL:-/);
  assert.match(launcher, /NVBES_IDENTITY_WORKER_INLINE_HOUSEKEEPING_ENABLED:-false/);
});

test('identity service command starts the complete email delivery runtime', async () => {
  const packageJson = JSON.parse(await workspaceFile('package.json'));
  assert.equal(packageJson.scripts['dev:identity-service'], 'bash scripts/dev-identity.sh');

  const launcher = await workspaceFile('scripts/dev-identity.sh');
  for (const dependency of [
    'dev-email-worker.sh',
    'dev-identity-worker.sh',
    'dev-identity-service.sh',
  ]) {
    assert.match(launcher, new RegExp(dependency.replace('.', '\\.')));
  }
});

test('email worker reloads when its renderer or generated React templates change', async () => {
  const launcher = await workspaceFile('scripts/dev-email-worker.sh');

  assert.match(launcher, /cargo watch/);
  assert.match(launcher, /--watch libs\/rust\/email/);
  assert.match(launcher, /--watch apps\/email-worker/);
  assert.match(launcher, /pnpm --dir libs\/ts\/email-ui run generate/);
  assert.doesNotMatch(
    launcher,
    /--shell '[^']*pnpm generate:email/,
    'the cargo watcher must not start Nx inside a detached PTY',
  );
  assert.match(launcher, /run -p nvbes-email-worker/);
});

test('development environment loaders expose custom Node debugger names', async () => {
  const [testEnv, workspaceEnv] = await Promise.all([
    workspaceFile('scripts/lib/test-env.sh'),
    workspaceFile('scripts/lib/workspace-env.sh'),
  ]);

  assert.match(testEnv, /nvbes_node_process_title="\$\{NVBES_NODE_PROCESS_TITLE:-nvbes:/);
  assert.match(testEnv, /load_workspace_env "\$nvbes_node_process_title"/);
  assert.match(workspaceEnv, /node --title="\$node_process_title"/);
  assert.match(
    workspaceEnv,
    /NVBES_DEVCONTAINER_IDENTITY_TEST_DATABASE_URL:-postgres:\/\/postgres:postgres@postgres:5432\/nvbes_identity_test/,
  );
});

test('email database tests load the worker database environment', async () => {
  const [project, launcher] = await Promise.all([
    workspaceFile('apps/email-worker/project.json'),
    workspaceFile('scripts/test-email-worker-database.sh'),
  ]);
  const configuration = JSON.parse(project);

  assert.equal(
    configuration.targets['test:database'].options.command,
    'bash scripts/test-email-worker-database.sh',
  );
  assert.match(launcher, /source "\$ROOT_DIR\/scripts\/lib\/test-env\.sh"/);
  assert.match(
    launcher,
    /export DATABASE_URL="\$\{DATABASE_URL:-\$\{NVBES_EMAIL_DATABASE_URL:-\}\}"/,
  );
  assert.match(launcher, /validate-email-test-database\.mjs/);
  assert.match(launcher, /--test-threads=1/);
});

test('email database tests accept only local or devcontainer email databases', () => {
  assert.deepEqual(
    validateEmailTestDatabaseTarget({
      DATABASE_URL: 'postgres://postgres:postgres@localhost:15432/nvbes_email',
      NVBES_ENVIRONMENT: 'development',
    }),
    { databaseName: 'nvbes_email', hostname: 'localhost' },
  );
  assert.deepEqual(
    validateEmailTestDatabaseTarget({
      DATABASE_URL: 'postgres://postgres:postgres@postgres:5432/nvbes_email_test',
      NVBES_DEVCONTAINER: 'true',
      NVBES_ENV: 'test',
    }),
    { databaseName: 'nvbes_email_test', hostname: 'postgres' },
  );

  for (const environment of [
    {
      DATABASE_URL: 'postgres://nvbes@example.com/nvbes_email',
      NVBES_ENV: 'test',
    },
    {
      DATABASE_URL: 'postgres://postgres@localhost/nvbes',
      NVBES_ENV: 'test',
    },
    {
      DATABASE_URL: 'postgres://postgres@localhost/nvbes_email',
      NVBES_ENVIRONMENT: 'production',
    },
  ]) {
    assert.throws(() => validateEmailTestDatabaseTarget(environment));
  }
});

test('identity migration tests prefer the dedicated test database', async () => {
  const launcher = await workspaceFile('scripts/test-identity-service-migrations.sh');

  assert.match(launcher, /source "\$ROOT_DIR\/scripts\/lib\/test-env\.sh"/);
  assert.match(launcher, /NVBES_IDENTITY_TEST_DATABASE_URL:-\$\{NVBES_IDENTITY_DATABASE_URL:-\}/);
});
