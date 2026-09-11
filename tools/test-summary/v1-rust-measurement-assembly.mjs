import assert from 'node:assert/strict';
import { sha256 } from './v1-bundle-verification.mjs';
import { rustCoverage, rustMutation } from './v1-rust-measurements.mjs';

const REPOSITORY = 'nvbes-org/nvbes';
const canonicalDate = (value) =>
  typeof value === 'string' &&
  Number.isFinite(Date.parse(value)) &&
  new Date(value).toISOString() === value;

export const rustMeasurementArtifact = (name, sha) => name === `v1-measurement-rust-${sha}`;

export function assembleRustMeasurements({ metadata, run, units, readArtifact }) {
  const id = Number(metadata.id);
  assert(Number.isSafeInteger(id) && id > 0, 'Invalid Rust artifact ID');
  const members = new Map();
  for (const name of ['measurements.json', 'llvm.json', 'mutation.json']) {
    const bytes = readArtifact({
      reference: { artifactId: id, member: name },
      run,
      sha: run.head_sha,
    });
    assert(Buffer.isBuffer(bytes) && bytes.length > 0, `Empty Rust ${name}`);
    members.set(name, bytes);
  }
  const measurements = JSON.parse(members.get('measurements.json').toString('utf8'));
  const llvm = JSON.parse(members.get('llvm.json').toString('utf8'));
  const mutation = JSON.parse(members.get('mutation.json').toString('utf8'));
  assert(Array.isArray(measurements), 'Invalid Rust measurement manifest');
  const rustUnits = units.filter((unit) => unit.language === 'rust');
  assert.equal(measurements.length, rustUnits.length, 'Incomplete Rust measurement manifest');
  const artifactEntries = [...members].map(([name, bytes]) => ({
    path: `rust/${name}`,
    sha256: sha256(bytes),
    ci: { artifactId: id, member: name },
  }));
  const files = new Map([...members].map(([name, bytes]) => [`rust/${name}`, bytes]));
  const normalized = [];
  for (const unit of rustUnits) {
    const measurement = measurements.find((entry) => entry.unit === unit.name);
    assert(measurement, `Missing Rust measurement: ${unit.name}`);
    assert.equal(measurement.language, 'rust');
    assert.equal(measurement.sha, run.head_sha);
    assert.equal(measurement.attempt, 1);
    assert.equal(measurement.measurementVersion, 1);
    assert(
      canonicalDate(measurement.completedAt) &&
        Date.parse(measurement.completedAt) >= Date.parse(run.run_started_at) &&
        Date.parse(measurement.completedAt) <= Date.parse(run.updated_at),
      'Rust measurement timestamp outside workflow',
    );
    assert(
      measurement.tools &&
        Object.values(measurement.tools).every(
          (value) => typeof value === 'string' && value.trim().length > 0,
        ),
      'Missing Rust measurement tool versions',
    );
    assert.deepEqual(measurement.producer, {
      kind: 'github-actions',
      repository: REPOSITORY,
      workflow: run.path,
      runId: run.id,
    });
    assert.deepEqual(measurement.reports, {
      'llvm-lines': 'rust/llvm.json',
      'llvm-branches': 'rust/llvm.json',
      'cargo-mutants': 'rust/mutation.json',
    });
    assert.deepEqual(
      measurement.artifacts,
      artifactEntries.map((entry) => entry.path),
    );
    assert.equal(measurement.lines, rustCoverage(unit, llvm, 'lines'));
    assert.equal(measurement.branches, rustCoverage(unit, llvm, 'branches'));
    assert.equal(measurement.mutation, rustMutation(unit, mutation));
    const candidate = { ...measurement };
    delete candidate.measurementVersion;
    normalized.push(candidate);
  }
  return { measurements: normalized, artifacts: artifactEntries, files };
}
