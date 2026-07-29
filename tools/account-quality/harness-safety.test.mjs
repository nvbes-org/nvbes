import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';

test('test harnesses never pass the beta password through argv', async () => {
  const [critical, betaCli, openApiSmoke, seededAuth, packageJson] = await Promise.all([
    readFile('apps/account-web/e2e/critical.spec.ts', 'utf8'),
    readFile('apps/account-service/src/identity.tools.beta.rs', 'utf8'),
    readFile('scripts/test-openapi-contract.mjs', 'utf8'),
    readFile('scripts/lib/openapi-seeded-auth.mjs', 'utf8'),
    readFile('package.json', 'utf8'),
  ]);

  assert.doesNotMatch(critical, /['"]--password['"]/u);
  assert.match(critical, /NVBES_BETA_SEED_PASSWORD:\s*credentials\.betaPassword/u);
  assert.match(critical, /catch\s*\{\s*throw new Error\(['"]Beta account preparation failed\./u);
  assert.doesNotMatch(betaCli, /extract_arg_value\(args,\s*["']--password["']/u);
  assert.match(betaCli, /std::env::var\(["']NVBES_BETA_SEED_PASSWORD["']\)/u);
  assert.doesNotMatch(openApiSmoke, /['"]--password['"]/u);
  assert.doesNotMatch(seededAuth, /['"]--password['"]/u);
  assert.match(seededAuth, /NVBES_BETA_SEED_PASSWORD:\s*credentials\.password/u);
  assert.doesNotMatch(packageJson, /beta:seed:staging/u);
});

test('migration harness preserves non-zero signal exit codes after cleanup', async () => {
  const runner = await readFile('scripts/test-account-service-migrations.sh', 'utf8');

  assert.match(runner, /trap finalize_exit EXIT/u);
  assert.match(runner, /trap 'finalize_signal 130' INT/u);
  assert.match(runner, /trap 'finalize_signal 143' TERM/u);
  assert.match(runner, /finalize_signal\(\)[\s\S]+cleanup[\s\S]+exit "\$signal_status"/u);
  assert.doesNotMatch(runner, /trap finalize EXIT INT TERM/u);
});

test('runtime OpenAPI contract is standalone and infrastructure-free', async () => {
  const runner = await readFile('scripts/test-account-service-contract.sh', 'utf8');

  assert.match(runner, /cargo run[\s\S]+--export-openapi/u);
  assert.match(runner, /ACCOUNT_RUNTIME_OPENAPI="\$runtime_openapi"/u);
  assert.doesNotMatch(runner, /cargo test/u);
  assert.doesNotMatch(runner, /(?:DATABASE|REDIS)_URL/u);
});

test('database-backed Account service gates share fail-closed infrastructure preflight', async () => {
  const [service, security, ci] = await Promise.all([
    readFile('scripts/test-account-service.sh', 'utf8'),
    readFile('scripts/test-account-service-security.sh', 'utf8'),
    readFile('.github/workflows/ci.yml', 'utf8'),
  ]);
  const cargo = service.indexOf('exec cargo test');

  assert.ok(service.indexOf('source "$ROOT_DIR/scripts/lib/test-env.sh"') < cargo);
  assert.ok(service.indexOf('require_destructive_account_test_database') < cargo);
  assert.ok(service.indexOf('require_account_test_redis') < cargo);
  assert.match(security, /exec bash "\$ROOT_DIR\/scripts\/test-account-service\.sh"/u);
  assert.match(ci, /NVBES_ENV: ci/u);
  assert.match(ci, /NVBES_REDIS_URL: redis:\/\/127\.0\.0\.1:6379\/15/u);
});

test('critical E2E validates every mutable target before starting Playwright', async () => {
  const runner = await readFile('scripts/test-e2e-critical.sh', 'utf8');
  const webPolicy = runner.indexOf('validate-load-target.mjs" web');
  const servicePolicy = runner.indexOf('validate-load-target.mjs" service');
  const cloudPolicy = runner.indexOf('validate-load-target.mjs" cloud');
  const playwright = runner.indexOf('test:e2e:critical');

  for (const policy of [webPolicy, servicePolicy, cloudPolicy]) {
    assert.ok(policy >= 0, 'target policy invocation is missing');
    assert.ok(policy < playwright, 'target policy must run before Playwright');
  }
  assert.match(runner, /NVBES_ENV="\$NVBES_TARGET_ENV" require_destructive_account_test_database/u);
});

test('staging release entry points validate exact target allowlists before use', async () => {
  const [releaseGate, stagingSmoke, helpers] = await Promise.all([
    readFile('scripts/release-gate.sh', 'utf8'),
    readFile('scripts/smoke-staging.sh', 'utf8'),
    readFile('scripts/lib/test-env.sh', 'utf8'),
  ]);

  assert.match(releaseGate, /validate_staging_account_targets/u);
  assert.match(stagingSmoke, /validate_staging_account_targets/u);
  for (const variable of [
    'NVBES_STAGING_ALLOWED_WEB_ORIGINS',
    'NVBES_STAGING_ALLOWED_API_ORIGINS',
    'ACCOUNT_PRODUCTION_DENIED_ORIGINS',
  ]) {
    assert.match(helpers, new RegExp(`require_env ${variable}\\b`, 'u'));
  }
  assert.match(helpers, /NVBES_TARGET_ENV=staging[\s\S]+validate-load-target\.mjs" web/u);
  assert.match(helpers, /NVBES_TARGET_ENV=staging[\s\S]+validate-load-target\.mjs" service/u);
});

test('OpenAPI gate never retries a failed probe into a green result', async () => {
  const [smoke, seededAuth] = await Promise.all([
    readFile('scripts/test-openapi-contract.mjs', 'utf8'),
    readFile('scripts/lib/openapi-seeded-auth.mjs', 'utf8'),
  ]);
  const probe = smoke.slice(
    smoke.indexOf('async function probe('),
    smoke.indexOf('async function main()'),
  );

  assert.doesNotMatch(probe, /for\s*\([^)]*attempt/u);
  assert.doesNotMatch(probe, /UND_ERR_SOCKET/u);
  assert.equal((probe.match(/await fetch\(/gu) ?? []).length, 1);
  assert.doesNotMatch(seededAuth, /attempt \+\s*1\}\/3/u);
  assert.doesNotMatch(seededAuth, /setTimeout\([^)]*attempt/u);
});

test('public smoke requires metrics authentication instead of public exposure', async () => {
  const smoke = await readFile('scripts/test-smoke.sh', 'utf8');

  assert.match(smoke, /metrics_status=.*http_status "\$API_BASE_URL\/metrics"/u);
  assert.match(smoke, /401 \| 403/u);
  assert.doesNotMatch(smoke, /metrics_body=.*http_body "\$API_BASE_URL\/metrics"/u);
});

test('nightly runner loads the shared fail-closed harness before using it', async () => {
  const nightly = await readFile('tools/account-quality/run-account-nightly.sh', 'utf8');
  const source = nightly.indexOf('source "$workspace_root/scripts/lib/test-env.sh"');
  const guard = nightly.indexOf('require_destructive_account_test_database');

  assert.ok(source >= 0, 'nightly runner must source test-env.sh');
  assert.ok(guard > source, 'destructive database guard must run after helper loading');
});

test('workspace env provides defaults without overriding explicit CI targets', async () => {
  const fixtureRoot = await mkdtemp(path.join(tmpdir(), 'nvbes-workspace-env-'));
  try {
    await writeFile(
      path.join(fixtureRoot, '.env'),
      [
        'NVBES_DATABASE_URL=postgres://local-file/unsafe',
        'NVBES_REDIS_URL=redis://local-file:6379',
        'NVBES_FILE_ONLY_DEFAULT=loaded',
      ].join('\n'),
      { mode: 0o600 },
    );

    const output = execFileSync(
      'bash',
      [
        '-c',
        [
          'set -euo pipefail',
          'source scripts/lib/workspace-env.sh',
          'load_workspace_env',
          'printf "%s\\n%s\\n%s\\n" "$NVBES_DATABASE_URL" "$NVBES_REDIS_URL" "$NVBES_FILE_ONLY_DEFAULT"',
        ].join('; '),
      ],
      {
        cwd: process.cwd(),
        encoding: 'utf8',
        env: {
          ...process.env,
          ROOT_DIR: fixtureRoot,
          NVBES_DATABASE_URL: 'postgres://explicit/test',
          NVBES_REDIS_URL: 'redis://explicit:6379/15',
        },
      },
    );

    assert.deepEqual(output.trim().split('\n'), [
      'postgres://explicit/test',
      'redis://explicit:6379/15',
      'loaded',
    ]);
  } finally {
    await rm(fixtureRoot, { recursive: true, force: true });
  }
});
