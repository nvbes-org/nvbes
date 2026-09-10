import assert from 'node:assert/strict';
import { artifactId } from './v1-ci-artifacts.mjs';
import { sha256 } from './v1-bundle-verification.mjs';
import { typescriptMeasurementSlug } from './v1-measurement-receipt.mjs';
import { typescriptMeasurements } from './v1-typescript-measurements.mjs';

const REPOSITORY = 'nvbes-org/nvbes';
const canonicalDate = (value) =>
  typeof value === 'string' &&
  Number.isFinite(Date.parse(value)) &&
  new Date(value).toISOString() === value;

export function typescriptMeasurementArtifact(name, sha) {
  const prefix = 'v1-measurement-typescript-';
  const suffix = `-${sha}`;
  if (!name?.startsWith(prefix) || !name.endsWith(suffix)) return null;
  const slug = name.slice(prefix.length, -suffix.length);
  assert(/^[a-z][a-z0-9-]*$/u.test(slug), 'Invalid TypeScript measurement artifact name');
  return slug;
}

function parse(bytes, label) {
  assert(Buffer.isBuffer(bytes) && bytes.length > 0, `Empty ${label}`);
  try {
    return JSON.parse(bytes.toString('utf8'));
  } catch {
    throw new Error(`Invalid ${label} JSON`);
  }
}

export function assembleTypescriptMeasurement({ metadata, slug, run, units, readArtifact }) {
  const id = artifactId(metadata.id);
  const reference = (member) => ({ artifactId: Number(id), member });
  const receiptBytes = readArtifact({
    reference: reference('measurement.json'),
    run,
    sha: run.head_sha,
  });
  const receipt = parse(receiptBytes, 'measurement receipt');
  const unit = units.find((entry) => entry.name === receipt.unit);
  assert(unit?.language === 'typescript', 'Unknown TypeScript measurement unit');
  assert.equal(typescriptMeasurementSlug(unit), slug, 'Measurement artifact unit mismatch');
  assert.equal(receipt.measurementVersion, 1, 'Unsupported measurement receipt version');
  assert.equal(receipt.language, 'typescript', 'Measurement language mismatch');
  assert.equal(receipt.sha, run.head_sha, 'Measurement SHA mismatch');
  assert.equal(receipt.attempt, 1, 'Measurement retry forbidden');
  assert(
    canonicalDate(receipt.completedAt) &&
      Date.parse(receipt.completedAt) >= Date.parse(run.run_started_at) &&
      Date.parse(receipt.completedAt) <= Date.parse(run.updated_at),
    'Measurement timestamp outside workflow',
  );
  assert(
    receipt.tools &&
      Object.keys(receipt.tools).length > 0 &&
      Object.values(receipt.tools).every((value) => typeof value === 'string' && value.trim()),
    'Missing measurement tool versions',
  );
  assert.deepEqual(
    receipt.producer,
    {
      kind: 'github-actions',
      repository: REPOSITORY,
      workflow: run.path,
      runId: run.id,
    },
    'Measurement producer mismatch',
  );
  assert.deepEqual(
    receipt.reports,
    { 'istanbul-summary': 'coverage.json', stryker: 'mutation.json' },
    'Measurement reports mismatch',
  );
  assert.deepEqual(
    receipt.artifacts,
    ['measurement.json', 'coverage.json', 'mutation.json'],
    'Measurement artifacts mismatch',
  );
  const coverageBytes = readArtifact({
    reference: reference('coverage.json'),
    run,
    sha: run.head_sha,
  });
  const mutationBytes = readArtifact({
    reference: reference('mutation.json'),
    run,
    sha: run.head_sha,
  });
  const scores = typescriptMeasurements(
    unit,
    parse(coverageBytes, 'coverage report'),
    parse(mutationBytes, 'mutation report'),
  );
  for (const [metric, score] of Object.entries(scores)) {
    assert.equal(receipt[metric], score, `Declared ${metric} differs from raw report`);
  }
  const prefix = `measurements/typescript-${slug}`;
  const members = new Map([
    [`${prefix}/measurement.json`, receiptBytes],
    [`${prefix}/coverage.json`, coverageBytes],
    [`${prefix}/mutation.json`, mutationBytes],
  ]);
  const artifacts = [...members].map(([path, bytes]) => ({
    path,
    sha256: sha256(bytes),
    ci: reference(path.slice(prefix.length + 1)),
  }));
  const measurement = { ...receipt };
  delete measurement.measurementVersion;
  measurement.reports = {
    'istanbul-summary': `${prefix}/coverage.json`,
    stryker: `${prefix}/mutation.json`,
  };
  measurement.artifacts = [...members.keys()];
  return { id, unit: unit.name, measurement, artifacts, files: members };
}
