import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

import { validateEmailTestDatabaseTarget } from './validate-email-test-database.mjs';

const root = new URL('../', import.meta.url);

async function workspaceFile(path) {
  return readFile(new URL(path, root), 'utf8');
}

void test('email worker reloads when its renderer or generated React templates change', async () => {
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
  assert.match(launcher, /NVBES_DEV_EMAIL_BIND_ADDR/);
  assert.match(launcher, /NVBES_EMAIL_GRPC_BIND_ADDR="\$NVBES_EMAIL_HTTP_BIND_ADDR"/);
});

void test('the root development launcher starts the complete V1 ecosystem', async () => {
  const launcher = await workspaceFile('scripts/dev.sh');

  assert.match(launcher, /docker compose .* up --detach --wait/);
  assert.match(launcher, /dev-email-worker\.sh/);
  assert.match(launcher, /dev-trust-risk-service\.sh/);
  assert.match(launcher, /dev-identity-service\.sh/);
  assert.match(launcher, /dev-account-service\.sh/);
  assert.match(launcher, /dev-billing-service\.sh/);
  assert.match(
    launcher,
    /for database in nvbes_dev_email nvbes_dev_trust_risk nvbes_dev_identity nvbes_dev_account nvbes_dev_billing/,
  );
  assert.match(launcher, /createdb --username postgres "\$database"/);
  assert.match(launcher, /cargo run --package nvbes-email-worker -- migrate/);
  assert.match(launcher, /cargo run --package nvbes-trust-risk-service -- migrate/);
  assert.match(launcher, /cargo run --package nvbes-identity-service -- migrate/);
  assert.match(launcher, /cargo run --package nvbes-account-service -- migrate/);
  assert.match(
    launcher,
    /cargo run --manifest-path apps\/billing-service\/Cargo\.toml -- migrate/,
  );
  assert.doesNotMatch(launcher, /dev-(cloud|enterprise|developer|backoffice)/);
});

void test('local Identity consumers share one generated RSA public key', async () => {
  const [keys, account, billing] = await Promise.all([
    workspaceFile('scripts/lib/dev-identity-keys.sh'),
    workspaceFile('scripts/dev-account-service.sh'),
    workspaceFile('scripts/dev-billing-service.sh'),
  ]);

  assert.match(keys, /\.temp\/dev-runtime/);
  assert.match(keys, /openssl genpkey -algorithm RSA/);
  assert.match(keys, /NVBES_IDENTITY_PUBLIC_KEY_PEM/);
  assert.match(account, /load_dev_identity_keys/);
  assert.match(billing, /load_dev_identity_keys/);
  assert.match(
    billing,
    /--manifest-path apps\/billing-service\/Cargo\.toml -- serve/,
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

void test('email database tests load the worker database environment', async () => {
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
