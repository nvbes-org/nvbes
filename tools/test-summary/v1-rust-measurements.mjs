import assert from 'node:assert/strict';

function sourcePath(filename) {
  assert(typeof filename === 'string' && filename.length > 0, 'Missing Rust source path');
  const file = filename.replaceAll('\\', '/');
  assert(!file.split('/').includes('..'), 'Rust report path traversal');
  return file;
}

function memberSource(filename, unit) {
  const file = sourcePath(filename);
  const prefix = `${unit.sourceRoot}/`;
  const start = file.startsWith(prefix) ? 0 : file.indexOf(`/${prefix}`) + 1;
  if (start === 0 && !file.startsWith(prefix)) return null;
  const relative = file.slice(start);
  assert(unit.sources.includes(relative), `Unknown candidate Rust source: ${relative}`);
  return relative;
}

export function rustCoverage(unit, report, metric) {
  assert(['lines', 'branches'].includes(metric), 'Invalid Rust coverage metric');
  assert.equal(report.type, 'llvm.coverage.json.export', 'Unsupported LLVM report type');
  assert(['3.0.1', '3.1.0'].includes(report.version), 'Unsupported LLVM report version');
  assert(Array.isArray(report.data) && report.data.length === 1, 'Expected one merged LLVM export');
  assert(Array.isArray(report.data[0].files), 'Missing LLVM files');
  const seen = new Set();
  let total = 0;
  let covered = 0;
  for (const entry of report.data[0].files) {
    const file = memberSource(entry.filename, unit);
    if (!file || /\.(tests|test_support)\.rs$/u.test(file)) continue;
    assert(!seen.has(file), `Duplicate LLVM source: ${file}`);
    seen.add(file);
    const counters = entry.summary?.[metric];
    assert(
      counters &&
        Number.isSafeInteger(counters.count) &&
        Number.isSafeInteger(counters.covered) &&
        counters.count >= 0 &&
        counters.covered >= 0 &&
        counters.covered <= counters.count,
      `Invalid LLVM ${metric} counters: ${file}`,
    );
    total += counters.count;
    covered += counters.covered;
  }
  assert(
    Number.isSafeInteger(total) && total > 0,
    `${unit.name}: no applicable ${metric} counters`,
  );
  return (100 * covered) / total;
}

function successfulTest(outcome) {
  return outcome.phase_results?.some(
    (phase) => phase.phase === 'Test' && phase.process_status === 'Success',
  );
}

function validatePhases(outcome) {
  assert(Array.isArray(outcome.phase_results), 'Missing mutation execution phases');
  const phases = new Map();
  for (const phase of outcome.phase_results) {
    assert(
      ['Build', 'Test'].includes(phase.phase) && !phases.has(phase.phase),
      'Invalid mutation phases',
    );
    phases.set(phase.phase, phase.process_status);
  }
  const failed = (status) => Number.isSafeInteger(status?.Failure) && status.Failure > 0;
  const build = phases.get('Build');
  const test = phases.get('Test');
  const consistent = {
    Success: build === 'Success' && test === 'Success',
    CaughtMutant: build === 'Success' && failed(test),
    MissedMutant: build === 'Success' && test === 'Success',
    Timeout:
      (build === 'Timeout' && test === undefined) || (build === 'Success' && test === 'Timeout'),
    Unviable: failed(build) && test === undefined,
  };
  assert(consistent[outcome.summary] === true, 'Mutation summary contradicts execution phases');
}

export function rustMutation(unit, report) {
  assert.equal(report.cargo_mutants_version, '27.1.0', 'Unsupported cargo-mutants version');
  assert(Array.isArray(report.outcomes), 'Missing cargo-mutants outcomes');
  assert(
    Number.isFinite(Date.parse(report.start_time)) &&
      Number.isFinite(Date.parse(report.end_time)) &&
      Date.parse(report.end_time) >= Date.parse(report.start_time),
    'Incomplete mutation campaign',
  );
  const baselines = report.outcomes.filter((outcome) => outcome.scenario === 'Baseline');
  assert(
    baselines.length === 1 && baselines[0].summary === 'Success' && successfulTest(baselines[0]),
    'Missing or unsuccessful mutation baseline',
  );
  validatePhases(baselines[0]);
  const seen = new Set();
  let caught = 0;
  let tested = 0;
  let mutants = 0;
  for (const outcome of report.outcomes) {
    if (outcome.scenario === 'Baseline') continue;
    const mutant = outcome.scenario?.Mutant;
    assert(mutant && typeof mutant.package === 'string', 'Invalid mutant scenario');
    const file =
      mutant.package === unit.name ? memberSource(mutant.file, unit) : sourcePath(mutant.file);
    assert(file, 'Mutant outside production crate');
    const span = mutant.span;
    for (const point of [span?.start, span?.end]) {
      assert(
        Number.isSafeInteger(point?.line) &&
          point.line > 0 &&
          Number.isSafeInteger(point?.column) &&
          point.column > 0,
        'Invalid mutant position',
      );
    }
    assert(
      span.end.line > span.start.line ||
        (span.end.line === span.start.line && span.end.column >= span.start.column),
      'Reversed mutant span',
    );
    assert(typeof mutant.replacement === 'string', 'Missing mutant replacement');
    const key = JSON.stringify([
      mutant.package,
      file,
      span.start.line,
      span.start.column,
      span.end.line,
      span.end.column,
      mutant.replacement,
    ]);
    assert(!seen.has(key), 'Duplicate Rust mutant');
    seen.add(key);
    assert(
      ['CaughtMutant', 'MissedMutant', 'Timeout', 'Unviable'].includes(outcome.summary),
      'Unsupported or incomplete Rust mutation result',
    );
    validatePhases(outcome);
    mutants++;
    if (mutant.package !== unit.name) continue;
    const candidate = memberSource(file, unit);
    assert(
      candidate && !/\.(tests|test_support)\.rs$/u.test(candidate),
      'Mutant outside production crate',
    );
    if (outcome.summary === 'Unviable') continue;
    tested++;
    if (outcome.summary === 'CaughtMutant') caught++;
  }
  assert.equal(report.total_mutants, mutants, 'Incomplete Rust mutant count');
  assert(tested > 0, `${unit.name}: no viable Rust mutants`);
  return (100 * caught) / tested;
}

export function verifyRustMeasurements(bundle, units, artifacts) {
  for (const unit of units.filter((entry) => entry.language === 'rust')) {
    assert(
      typeof unit.sourceRoot === 'string' && Array.isArray(unit.sources) && unit.sources.length,
      'Missing candidate Rust source inventory',
    );
    const matches = bundle.measurements?.filter((entry) => entry.unit === unit.name);
    assert.equal(matches?.length, 1, `Missing or duplicate measurement: ${unit.name}`);
    const measurement = matches[0];
    const read = (format) => {
      const file = measurement.reports?.[format];
      assert(
        typeof file === 'string' && measurement.artifacts?.includes(file) && artifacts.has(file),
        `Missing verified ${format} report: ${unit.name}`,
      );
      return JSON.parse(artifacts.get(file).toString('utf8'));
    };
    const scores = {
      lines: rustCoverage(unit, read('llvm-lines'), 'lines'),
      branches: rustCoverage(unit, read('llvm-branches'), 'branches'),
      mutation: rustMutation(unit, read('cargo-mutants')),
    };
    for (const [metric, score] of Object.entries(scores)) {
      assert.equal(
        measurement[metric],
        score,
        `${unit.name}: declared ${metric} differs from raw report`,
      );
    }
  }
}
