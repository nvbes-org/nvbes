import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { artifactId, githubArtifactReader } from './v1-ci-artifacts.mjs';
import { sha256 } from './v1-bundle-verification.mjs';
import { productionUnits } from './v1-catalogue.mjs';
import {
  assembleTypescriptMeasurement,
  typescriptMeasurementArtifact,
} from './v1-measurement-assembly.mjs';
import { loadV1 } from './v1-release.mjs';
import { eligibleSuite, suiteCommandDigest } from './v1-suite-receipt.mjs';

const REPOSITORY = 'nvbes-org/nvbes';
const WORKFLOWS = ['.github/workflows/ci.yml', '.github/workflows/v1-testing.yml'];
const canonicalDate = (value) =>
  typeof value === 'string' &&
  Number.isFinite(Date.parse(value)) &&
  new Date(value).toISOString() === value;

export function validateAssemblyRun(run, runId) {
  assert.equal(String(run.id), artifactId(runId), 'Wrong workflow run ID');
  assert(
    Number.isSafeInteger(run.repository?.id) && run.repository.id > 0,
    'Missing repository ID',
  );
  assert.equal(run.repository.full_name, REPOSITORY, 'Wrong producer repository');
  assert(/^[a-f0-9]{40}$/u.test(run.head_sha ?? ''), 'Invalid workflow SHA');
  assert(WORKFLOWS.includes(run.path), 'Untrusted workflow');
  assert.equal(run.status, 'completed', 'Workflow is not complete');
  assert.equal(run.conclusion, 'success', 'Workflow did not succeed');
  assert.equal(run.run_attempt, 1, 'Rerun cannot produce release evidence');
  assert(['push', 'workflow_dispatch'].includes(run.event), 'Untrusted workflow event');
  assert(
    typeof run.run_started_at === 'string' &&
      typeof run.updated_at === 'string' &&
      Number.isFinite(Date.parse(run.run_started_at)) &&
      Number.isFinite(Date.parse(run.updated_at)) &&
      Date.parse(run.updated_at) >= Date.parse(run.run_started_at),
    'Invalid workflow timestamps',
  );
}

function receiptSuite(name, sha) {
  const suffix = `-${sha}`;
  if (!name?.startsWith('v1-receipt-') || !name.endsWith(suffix)) return null;
  const suite = name.slice('v1-receipt-'.length, -suffix.length);
  assert(/^[a-z0-9-]+\.[a-z0-9-]+$/u.test(suite), 'Invalid V1 receipt artifact name');
  return suite;
}

export function validateReceipt(receipt, suiteId, domains, run) {
  const selected = eligibleSuite(domains, suiteId);
  assert.equal(receipt.receiptVersion, 1, 'Unsupported receipt version');
  assert.equal(receipt.suite, suiteId, 'Receipt suite mismatch');
  assert.equal(receipt.domain, selected.domain, 'Receipt domain mismatch');
  assert.equal(receipt.sha, run.head_sha, 'Receipt SHA mismatch');
  assert.equal(receipt.environment, 'isolated', 'Receipt environment mismatch');
  assert.equal(receipt.status, 'passed', 'Failed suite receipt');
  assert.equal(receipt.attempt, 1, 'Receipt retry forbidden');
  assert.equal(receipt.target, selected.suite.target, 'Receipt target mismatch');
  assert.equal(
    receipt.commandSha256,
    suiteCommandDigest(selected.suite.target),
    'Receipt command mismatch',
  );
  assert(
    receipt.tools &&
      Object.keys(receipt.tools).length > 0 &&
      Object.values(receipt.tools).every((value) => typeof value === 'string' && value.trim()),
    'Missing receipt tool versions',
  );
  assert(
    canonicalDate(receipt.startedAt) &&
      canonicalDate(receipt.completedAt) &&
      Date.parse(receipt.startedAt) >= Date.parse(run.run_started_at) &&
      Date.parse(receipt.startedAt) <= Date.parse(receipt.completedAt) &&
      Date.parse(receipt.completedAt) <= Date.parse(run.updated_at),
    'Receipt timestamps outside workflow',
  );
  assert.deepEqual(
    receipt.producer,
    {
      kind: 'github-actions',
      repository: REPOSITORY,
      workflow: run.path,
      runId: run.id,
    },
    'Receipt producer mismatch',
  );
  assert.deepEqual(
    receipt.cases,
    selected.suite.cases.map((id) => ({ id, status: 'passed' })),
    'Incomplete receipt cases',
  );
  return receipt;
}

export function assembleReceiptDraft({
  run,
  runId,
  listing,
  domains,
  units = [],
  manifestDigest,
  readArtifact,
}) {
  validateAssemblyRun(run, runId);
  assert(/^[a-f0-9]{64}$/u.test(manifestDigest ?? ''), 'Invalid manifest digest');
  assert(
    Number.isSafeInteger(listing?.total_count) &&
      Array.isArray(listing.artifacts) &&
      listing.total_count === listing.artifacts.length &&
      listing.total_count <= 100,
    'Incomplete artifact listing',
  );
  const results = [];
  const measurements = [];
  const artifacts = [];
  const files = new Map();
  const ids = new Set();
  const suites = new Set();
  const measuredUnits = new Set();
  for (const metadata of listing.artifacts) {
    const suiteId = receiptSuite(metadata.name, run.head_sha);
    const measurementSlug = typescriptMeasurementArtifact(metadata.name, run.head_sha);
    if (!suiteId && !measurementSlug) continue;
    const id = artifactId(metadata.id);
    assert(!ids.has(id), 'Duplicate artifact ID');
    ids.add(id);
    if (measurementSlug) {
      const assembled = assembleTypescriptMeasurement({
        metadata,
        slug: measurementSlug,
        run,
        units,
        readArtifact,
      });
      assert(!measuredUnits.has(assembled.unit), 'Duplicate unit measurement');
      measuredUnits.add(assembled.unit);
      measurements.push(assembled.measurement);
      artifacts.push(...assembled.artifacts);
      for (const [file, bytes] of assembled.files) {
        assert(!files.has(file), 'Duplicate measurement file');
        files.set(file, bytes);
      }
      continue;
    }
    assert(!suites.has(suiteId), 'Duplicate suite receipt');
    suites.add(suiteId);
    const member = `${suiteId}.json`;
    const reference = { artifactId: Number(id), member };
    const bytes = readArtifact({ reference, run, sha: run.head_sha });
    assert(Buffer.isBuffer(bytes) && bytes.length > 0, 'Empty suite receipt');
    let receipt;
    try {
      receipt = JSON.parse(bytes.toString('utf8'));
    } catch {
      throw new Error('Invalid suite receipt JSON');
    }
    validateReceipt(receipt, suiteId, domains, run);
    const artifactPath = `receipts/${member}`;
    files.set(artifactPath, bytes);
    artifacts.push({ path: artifactPath, sha256: sha256(bytes), ci: reference });
    const result = { ...receipt };
    delete result.receiptVersion;
    results.push({ ...result, artifacts: [artifactPath] });
  }
  assert(results.length + measurements.length > 0, 'No V1 evidence artifacts found');
  results.sort((a, b) => a.suite.localeCompare(b.suite));
  measurements.sort((a, b) => a.unit.localeCompare(b.unit));
  artifacts.sort((a, b) => a.path.localeCompare(b.path));
  return {
    draft: {
      draftVersion: 1,
      incomplete: true,
      schemaVersion: 1,
      sha: run.head_sha,
      manifestDigest,
      results,
      artifacts,
      measurements,
      operations: null,
    },
    files,
  };
}

function api(endpoint) {
  try {
    return JSON.parse(
      execFileSync('gh', ['api', '--hostname', 'github.com', endpoint], {
        encoding: 'utf8',
        timeout: 30000,
        maxBuffer: 16 * 1024 * 1024,
        stdio: ['ignore', 'pipe', 'pipe'],
      }),
    );
  } catch {
    throw new Error('Unable to retrieve trusted workflow run from GitHub');
  }
}

function main() {
  assert(process.argv.length === 3, 'usage: v1-receipt-assembly.mjs --runId=<run-id>');
  assert(process.argv[2].startsWith('--runId='), 'Run ID must be a named argument');
  const runId = artifactId(process.argv[2].slice('--runId='.length));
  const cwd = process.cwd();
  const { root, domains, manifestDigest } = loadV1(cwd);
  const units = productionUnits(root, domains, cwd);
  const run = api(`repos/${REPOSITORY}/actions/runs/${runId}`);
  const listing = api(`repos/${REPOSITORY}/actions/runs/${runId}/artifacts?per_page=100`);
  const assembled = assembleReceiptDraft({
    run,
    runId,
    listing,
    domains,
    units,
    manifestDigest,
    readArtifact: githubArtifactReader(REPOSITORY),
  });
  const parent = path.join(cwd, '.temp/v1-drafts');
  const output = path.join(parent, runId);
  mkdirSync(parent, { recursive: true });
  mkdirSync(output);
  for (const [relative, bytes] of assembled.files) {
    const destination = path.join(output, relative);
    mkdirSync(path.dirname(destination), { recursive: true });
    writeFileSync(destination, bytes, { mode: 0o600, flag: 'wx' });
  }
  writeFileSync(path.join(output, 'draft.json'), `${JSON.stringify(assembled.draft, null, 2)}\n`, {
    mode: 0o600,
    flag: 'wx',
  });
  console.log(
    `Incomplete V1 draft assembled from ${assembled.draft.results.length} suite receipt(s) and ${assembled.draft.measurements.length} measurement(s): ${path.relative(cwd, output)}`,
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    console.error(`V1 receipt assembly refused: ${error.message}`);
    process.exitCode = 1;
  }
}
