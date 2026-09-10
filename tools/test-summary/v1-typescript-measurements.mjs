import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mutationScore } from './check-typescript-mutation.mjs';

const digest = (source) => createHash('sha256').update(source).digest('hex');

function filesInScope(files, unit, summary = false) {
  assert(files && typeof files === 'object' && !Array.isArray(files), 'Missing report files');
  const mapped = new Map();
  for (const [reported, value] of Object.entries(files)) {
    if (summary && reported === 'total') continue;
    const normalized = reported.replaceAll('\\', '/');
    assert(!normalized.split('/').includes('..'), 'Report path traversal');
    const matches = Object.keys(unit.sourceFiles).filter(
      (file) => normalized === file || normalized.endsWith(`/${file}`),
    );
    assert.equal(matches.length, 1, `File outside candidate scope: ${reported}`);
    const file = matches[0];
    assert(!mapped.has(file), `Duplicate source file: ${file}`);
    mapped.set(file, value);
  }
  for (const [file, source] of Object.entries(unit.sourceFiles)) {
    if (source.runtime) assert(mapped.has(file), `Missing runtime source: ${file}`);
  }
  return mapped;
}

export function typescriptMeasurements(unit, coverage, mutation) {
  assert(
    unit.sourceFiles && Object.keys(unit.sourceFiles).length,
    'Missing candidate source inventory',
  );
  const files = filesInScope(coverage, unit, true);
  const scores = {};
  for (const metric of ['lines', 'branches']) {
    let total = 0;
    let covered = 0;
    for (const [file, counters] of files) {
      const count = counters[metric];
      assert(
        count &&
          Number.isSafeInteger(count.total) &&
          Number.isSafeInteger(count.covered) &&
          count.total >= 0 &&
          count.covered >= 0 &&
          count.covered <= count.total &&
          count.skipped === 0,
        `Invalid ${metric} counters: ${file}`,
      );
      if (!unit.sourceFiles[file].runtime)
        assert.equal(count.total, 0, `Declarative source has counters: ${file}`);
      if (unit.sourceFiles[file].runtime && metric === 'lines')
        assert(count.total > 0, `Runtime source has no lines: ${file}`);
      total += count.total;
      covered += count.covered;
    }
    assert(Number.isSafeInteger(total) && total > 0, `No applicable ${metric} counters`);
    scores[metric] = (100 * covered) / total;
  }
  assert.equal(mutation.schemaVersion, '1.0', 'Unsupported Stryker report version');
  for (const [file, result] of filesInScope(mutation.files, unit)) {
    assert(typeof result.source === 'string', `Missing mutation source: ${file}`);
    assert.equal(
      digest(result.source),
      unit.sourceFiles[file].sha256,
      `Mutation source mismatch: ${file}`,
    );
    if (!unit.sourceFiles[file].runtime)
      assert.equal(result.mutants?.length, 0, `Declarative source has mutants: ${file}`);
  }
  scores.mutation = mutationScore(mutation).score;
  return scores;
}

// These bytes have passed signature/digest verification. CI collection still
// needs separate authentication; a signed report alone does not establish it.
export function verifyTypescriptMeasurements(bundle, units, artifacts) {
  for (const unit of units.filter((entry) => entry.language === 'typescript')) {
    const measurements = bundle.measurements?.filter((entry) => entry.unit === unit.name);
    assert.equal(measurements?.length, 1, `Missing or duplicate measurement: ${unit.name}`);
    const measurement = measurements[0];
    const readReport = (format) => {
      const file = measurement.reports?.[format];
      assert(
        typeof file === 'string' && measurement.artifacts?.includes(file) && artifacts.has(file),
        `Missing verified ${format} report: ${unit.name}`,
      );
      return JSON.parse(artifacts.get(file).toString('utf8'));
    };
    const scores = typescriptMeasurements(
      unit,
      readReport('istanbul-summary'),
      readReport('stryker'),
    );
    for (const [metric, score] of Object.entries(scores)) {
      assert.equal(
        measurement[metric],
        score,
        `${unit.name}: declared ${metric} differs from raw report`,
      );
    }
  }
}
