import { readFileSync } from 'node:fs';
import { aggregateCrateCoverage, loadCargoMetadata, percentOf } from './check-coverage.mjs';

export function evaluateCondition(report, packages, config) {
  if (config.schemaVersion !== 1)
    throw new Error(`Unsupported condition schemaVersion ${config.schemaVersion}`);
  const configuredNames = new Set(Object.keys(config.crates));
  const excludedNames = new Set(config.excluded.map((entry) => entry.crate));
  for (const name of configuredNames) {
    if (excludedNames.has(name)) throw new Error(`Crate ${name} is both configured and excluded`);
  }
  const memberNames = new Set(packages.map((pkg) => pkg.name));
  for (const name of [...configuredNames, ...excludedNames]) {
    if (!memberNames.has(name))
      throw new Error(`Configured crate ${name} is not a workspace member`);
  }
  for (const entry of config.excluded) {
    if (!entry.reason) throw new Error(`Excluded crate ${entry.crate} requires an explicit reason`);
    if (Number.isFinite(entry.baseline?.score) === false)
      throw new Error(`Excluded crate ${entry.crate} requires a recorded baseline score`);
  }

  const { aggregated } = aggregateCrateCoverage(report, packages);
  const rows = [];
  const failures = [];
  for (const [name, spec] of Object.entries(config.crates)) {
    const metric = aggregated.get(name);
    const branches = metric?.branches ?? [0, 0];
    const score = metric ? percentOf(branches) : 0;
    const gap = score < spec.threshold;
    rows.push({
      crate: name,
      present: Boolean(metric),
      count: branches[0],
      covered: branches[1],
      score,
      threshold: spec.threshold,
      gap,
    });
    if (gap) {
      failures.push(
        `${name}: condition coverage ${score.toFixed(1)}% (threshold ${spec.threshold}%)`,
      );
    }
  }
  return { rows, failures, aggregated };
}

export function formatConditionTable(rows) {
  const header = ['crate', 'branches', 'covered', 'score', 'threshold'];
  const lines = rows.map((row) => [
    row.crate,
    String(row.count),
    String(row.covered),
    row.present ? `${row.score.toFixed(1)}%` : 'absent',
    `${row.threshold}%`,
  ]);
  const widths = header.map((_, index) =>
    Math.max(header[index].length, ...lines.map((line) => line[index].length)),
  );
  const pad = (line) => header.map((_, index) => line[index].padEnd(widths[index])).join('  ');
  return [pad(header), lines.map(pad).join('\n')].join('\n');
}

const isCli = process.argv[1]?.endsWith('check-condition.mjs');
if (isCli) {
  const [reportPath, configPath = 'docs/testing/rust-condition-thresholds.json'] =
    process.argv.slice(2);
  if (!reportPath) throw new Error('usage: check-condition.mjs <llvm-cov-json> [thresholds.json]');
  const report = JSON.parse(readFileSync(reportPath, 'utf8'));
  const config = JSON.parse(readFileSync(configPath, 'utf8'));
  const packages = loadCargoMetadata();
  const result = evaluateCondition(report, packages, config);
  console.log(formatConditionTable(result.rows));
  if (result.failures.length) {
    console.error(`error: ${result.failures.length} condition coverage gap(s)`);
    for (const failure of result.failures) console.error(`  - ${failure}`);
    process.exit(1);
  }
}
