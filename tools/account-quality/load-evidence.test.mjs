import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const scriptRoot = path.dirname(fileURLToPath(import.meta.url));
const workspaceRoot = path.resolve(scriptRoot, '../..');
const node = process.execPath;

test('artifact validator rejects setup_data and a serialized cookie', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'account-evidence-'));
  const artifact = path.join(root, 'summary.json');
  const cookie = 'synthetic-cookie-that-must-not-leak';
  await writeFile(
    artifact,
    JSON.stringify({
      metrics: {},
      root_group: {},
      setup_data: { headers: { Cookie: cookie } },
    }),
  );

  const result = run('verify-load-artifact.mjs', [artifact], {
    ACCOUNT_TEST_COOKIE: cookie,
  });
  assert.notEqual(result.status, 0);
  assert.doesNotMatch(result.stderr, new RegExp(cookie, 'u'));
});

test('artifact validator accepts an empty setup result with metric evidence', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'account-evidence-'));
  const artifact = path.join(root, 'summary.json');
  await writeFile(
    artifact,
    JSON.stringify({
      metrics: { iterations: { count: 1 } },
      root_group: {},
      setup_data: null,
    }),
  );
  assert.equal(run('verify-load-artifact.mjs', [artifact]).status, 0);
});

test('metadata writer uses an allowlist and never serializes cookie environment data', async () => {
  const root = await mkdtemp(path.join(tmpdir(), 'account-metadata-'));
  const metadataPath = path.join(root, 'run.metadata.json');
  const cookie = 'synthetic-cookie-that-must-not-leak';
  const result = run('write-load-metadata.mjs', [metadataPath, 'summary.json'], {
    ACCOUNT_SERVICE_BASE_URL: 'http://127.0.0.1:4000',
    ACCOUNT_TEST_COOKIE: cookie,
    K6_PROFILE: 'smoke',
    K6_SUITE: 'account-session',
    NVBES_TARGET_ENV: 'ci',
  });
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(await readFile(metadataPath, 'utf8'), new RegExp(cookie, 'u'));
});

test('scalability gate fails closed without comparative evidence', () => {
  const result = run('check-scalability-comparison.mjs');
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /ACCOUNT_SCALABILITY_COMPARISON_FILE/u);
});

test('scalability gate rejects an otherwise valid campaign without trusted control-plane evidence', async () => {
  const { comparisonPath, root } = await writeScalabilityCampaign();
  const verdictPath = path.join(root, 'scalability-verdict.json');
  await writeFile(verdictPath, JSON.stringify({ verdict: 'pass' }));

  const result = run('check-scalability-comparison.mjs', [comparisonPath], {
    ACCOUNT_LOAD_ALLOWED_ORIGINS: 'https://account.staging.nvbes.fr',
    ACCOUNT_PRODUCTION_DENIED_ORIGINS: 'https://account.nvbes.fr,https://api.nvbes.fr',
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /no trusted control-plane attestation verifier/u);
  const verdict = JSON.parse(await readFile(verdictPath, 'utf8'));
  assert.equal(verdict.verdict, 'blocked');
  assert.equal(verdict.reason, 'control-plane-attestation-verifier-unavailable');
  assert.equal(verdict.reportedRuns[0].reportedReplicas, 1);
});

test('scalability gate rejects a duration other than the exact 15m contract', async () => {
  const { comparisonPath } = await writeScalabilityCampaign((metadata, replicas) => {
    if (replicas === 1) {
      metadata.configuredDuration = '14m';
    }
  });
  const result = run('check-scalability-comparison.mjs', [comparisonPath], {
    ACCOUNT_LOAD_ALLOWED_ORIGINS: 'https://account.staging.nvbes.fr',
    ACCOUNT_PRODUCTION_DENIED_ORIGINS: 'https://account.nvbes.fr,https://api.nvbes.fr',
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /configuredDuration must be exactly 15m/u);
});

test('scalability gate rejects a mutable or symbolic release identifier', async () => {
  const { comparisonPath } = await writeScalabilityCampaign((metadata, replicas) => {
    if (replicas === 2) {
      metadata.release = 'release-main';
    }
  });
  const result = run('check-scalability-comparison.mjs', [comparisonPath], {
    ACCOUNT_LOAD_ALLOWED_ORIGINS: 'https://account.staging.nvbes.fr',
    ACCOUNT_PRODUCTION_DENIED_ORIGINS: 'https://account.nvbes.fr,https://api.nvbes.fr',
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /immutable 7-64 character hexadecimal revision/u);
});

test('scalability gate rejects incomplete and extended metadata schemas', async () => {
  for (const mutation of [
    (metadata) => {
      delete metadata.generatedAt;
    },
    (metadata) => {
      metadata.selfDeclaredReadyReplicas = metadata.replicas;
    },
  ]) {
    const { comparisonPath } = await writeScalabilityCampaign((metadata, replicas) => {
      if (replicas === 4) {
        mutation(metadata);
      }
    });
    const result = run('check-scalability-comparison.mjs', [comparisonPath], {
      ACCOUNT_LOAD_ALLOWED_ORIGINS: 'https://account.staging.nvbes.fr',
      ACCOUNT_PRODUCTION_DENIED_ORIGINS: 'https://account.nvbes.fr,https://api.nvbes.fr',
    });
    assert.notEqual(result.status, 0);
    assert.match(result.stderr, /metadata must contain exactly/u);
  }
});

async function writeScalabilityCampaign(mutateMetadata = () => {}) {
  const root = await mkdtemp(path.join(tmpdir(), 'account-scalability-'));
  const comparison = { runs: [], schemaVersion: 1 };
  for (const [replicas, throughput] of [
    [1, 100],
    [2, 180],
    [4, 320],
    [8, 560],
  ]) {
    const metadataName = `replicas-${replicas}.metadata.json`;
    const summaryName = `replicas-${replicas}.json`;
    await writeFile(
      path.join(root, metadataName),
      JSON.stringify(
        applyMetadataMutation(
          {
            configuredDuration: '15m',
            configuredIterations: null,
            configuredLoadRate: throughput,
            datasetId: 'synthetic-scale-v1',
            generatedAt: '2026-07-29T10:00:00.000Z',
            profile: 'scalability',
            release: '0123456789abcdef',
            replicas,
            schemaVersion: 1,
            suite: 'account-api',
            summaryFile: summaryName,
            targetEnvironment: 'staging',
            targetOrigin: 'https://account.staging.nvbes.fr',
          },
          replicas,
          mutateMetadata,
        ),
      ),
    );
    await writeFile(
      path.join(root, summaryName),
      JSON.stringify({
        metrics: {
          checks: { value: 1 },
          dropped_iterations: { count: 0 },
          http_req_duration: { 'p(95)': 120, 'p(99)': 240 },
          http_req_failed: { value: 0 },
          iterations: { rate: throughput },
        },
        setup_data: null,
      }),
    );
    comparison.runs.push({ metadata: metadataName, summary: summaryName });
  }
  const comparisonPath = path.join(root, 'comparison.json');
  await writeFile(comparisonPath, JSON.stringify(comparison));
  return { comparisonPath, root };
}

function applyMetadataMutation(metadata, replicas, mutateMetadata) {
  mutateMetadata(metadata, replicas);
  return metadata;
}

test('runner rejects k6 reserved environment overrides before execution', () => {
  const result = spawnSync('bash', [path.join(scriptRoot, 'run-account-k6.sh')], {
    cwd: workspaceRoot,
    encoding: 'utf8',
    env: {
      ...process.env,
      ACCOUNT_SERVICE_BASE_URL: 'http://127.0.0.1:4000',
      K6_DURATION: '1s',
      K6_PROFILE: 'smoke',
      K6_SUITE: 'account-api',
      NVBES_TARGET_ENV: 'ci',
    },
  });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /K6_DURATION is reserved/u);
});

function run(script, args = [], environment = {}) {
  return spawnSync(node, [path.join(scriptRoot, script), ...args], {
    cwd: workspaceRoot,
    encoding: 'utf8',
    env: { ...process.env, ...environment },
  });
}
