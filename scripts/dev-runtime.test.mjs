import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import { validateEmailTestDatabaseTarget } from './validate-email-test-database.mjs';

const root = new URL('../', import.meta.url);

async function workspaceFile(path) {
  return readFile(new URL(path, root), 'utf8');
}

void test('API dev launcher watches API and shared Rust crates', async () => {
  const launcher = await workspaceFile('scripts/dev-api.sh');

  assert.match(launcher, /cargo watch/);
  assert.match(launcher, /--watch apps\/api/);
  assert.match(launcher, /--watch libs\/rust/);
  assert.match(launcher, /run -p nvbes-api/);
  assert.match(launcher, /PORT="\$\{PORT:-8080\}"/);
});

void test('Worker dev launcher watches Worker and shared Rust crates', async () => {
  const launcher = await workspaceFile('scripts/dev-worker.sh');

  assert.match(launcher, /cargo watch/);
  assert.match(launcher, /--watch apps\/worker/);
  assert.match(launcher, /--watch libs\/rust/);
  assert.match(launcher, /run -p nvbes-worker/);
  assert.match(launcher, /PORT="\$\{PORT:-8081\}"/);
});

void test('the root development launcher starts only API and Worker in V1 ecosystem', async () => {
  const launcher = await workspaceFile('scripts/dev.sh');

  assert.match(launcher, /docker compose .* up --detach --wait/);
  assert.match(launcher, /dev-api\.sh/);
  assert.match(launcher, /dev-worker\.sh/);
  assert.doesNotMatch(
    launcher,
    /dev-(email|trust-risk|identity-service|account-service|billing-service|cloud|enterprise|developer|backoffice)/,
  );
});

void test('development environment loaders expose custom Node debugger names', async () => {
  const [testEnv, workspaceEnv] = await Promise.all([
    workspaceFile('scripts/lib/test-env.sh'),
    workspaceFile('scripts/lib/workspace-env.sh'),
  ]);

  assert.match(testEnv, /nvbes_node_process_title="\$\{NVBES_NODE_PROCESS_TITLE:-nvbes:/);
  assert.match(testEnv, /load_workspace_env "\$nvbes_node_process_title"/);
  assert.match(workspaceEnv, /node --title="\$node_process_title"/);
  assert.match(
    workspaceEnv,
    /NVBES_DEVCONTAINER_EMAIL_DATABASE_URL:-postgres:\/\/postgres:postgres@postgres:5432\/nvbes_email/,
  );
});

void test('email database tests accept only local, devcontainer, or GitHub CI databases', () => {
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
  assert.deepEqual(
    validateEmailTestDatabaseTarget({
      DATABASE_URL: 'postgres://postgres:postgres@host.docker.internal:5432/nvbes_email_test',
      GITHUB_ACTIONS: 'true',
      NVBES_ENV: 'ci',
    }),
    { databaseName: 'nvbes_email_test', hostname: 'host.docker.internal' },
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
    {
      DATABASE_URL: 'postgres://postgres@host.docker.internal/nvbes_email_test',
      NVBES_ENV: 'ci',
    },
  ]) {
    assert.throws(() => validateEmailTestDatabaseTarget(environment));
  }
});
