import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { rustSources } from '../test-summary/v1-catalogue.mjs';
import { rustMutation } from '../test-summary/v1-rust-measurements.mjs';

export function aggregateMutationOutcomes(outcomes) {
  const perCrate = new Map();
  for (const outcome of outcomes) {
    const scenario = outcome.scenario;
    if (!scenario?.Mutant) continue;
    const pkg = scenario.Mutant.package;
    if (!perCrate.has(pkg)) {
      perCrate.set(pkg, {
        total: 0,
        caught: 0,
        missed: 0,
        timeout: 0,
        unviable: 0,
        tested: 0,
      });
    }
    const stat = perCrate.get(pkg);
    stat.total += 1;
    if (outcome.summary === 'CaughtMutant') {
      stat.caught += 1;
      stat.tested += 1;
    } else if (outcome.summary === 'MissedMutant') {
      stat.missed += 1;
      stat.tested += 1;
    } else if (outcome.summary === 'Timeout') {
      stat.timeout += 1;
      stat.tested += 1;
    } else if (outcome.summary === 'Unviable') {
      stat.unviable += 1;
    }
  }
  return perCrate;
}

export function mutationScore(stat) {
  if (stat.tested === 0) return 0;
  return (stat.caught * 100) / stat.tested;
}

function assertSupportedSummaries(outcomes) {
  const known = new Set(['Success', 'CaughtMutant', 'MissedMutant', 'Unviable', 'Timeout']);
  const unknown = outcomes
    .map((outcome) => outcome.summary)
    .filter((summary) => !known.has(summary));
  if (unknown.length)
    throw new Error(
      `Unsupported cargo-mutants outcome summary: ${[...new Set(unknown)].join(', ')}`,
    );
}

export function evaluateMutation(outcomes, config, packages) {
  if (config.schemaVersion !== 1)
    throw new Error(`Unsupported mutation schemaVersion ${config.schemaVersion}`);
  assertSupportedSummaries(outcomes);
  const configuredNames = new Set(Object.keys(config.crates));
  const excludedNames = new Set(config.excluded.map((entry) => entry.crate));
  for (const name of configuredNames) {
    if (excludedNames.has(name)) throw new Error(`Crate ${name} is both configured and excluded`);
  }
  const memberNames = new Set((packages ?? []).map((pkg) => pkg.name));
  if (packages) {
    for (const name of [...configuredNames, ...excludedNames]) {
      if (!memberNames.has(name))
        throw new Error(`Configured crate ${name} is not a workspace member`);
    }
  }
  for (const entry of config.excluded) {
    if (!entry.reason) throw new Error(`Excluded crate ${entry.crate} requires an explicit reason`);
    if (Number.isFinite(entry.baseline?.score) === false)
      throw new Error(`Excluded crate ${entry.crate} requires a recorded baseline score`);
  }
  if (configuredNames.size === 0) throw new Error('Empty mutation gate');
  for (const [name, spec] of Object.entries(config.crates)) {
    if (!Number.isFinite(spec.threshold) || spec.threshold <= 0 || spec.threshold > 100)
      throw new Error(`Invalid mutation threshold for ${name}`);
  }

  const stats = aggregateMutationOutcomes(outcomes);
  const rows = [];
  const failures = [];
  for (const [name, spec] of Object.entries(config.crates)) {
    const stat = stats.get(name);
    const score = stat ? mutationScore(stat) : 0;
    const gap = score < spec.threshold;
    rows.push({
      crate: name,
      present: Boolean(stat),
      total: stat?.total ?? 0,
      tested: stat?.tested ?? 0,
      caught: stat?.caught ?? 0,
      missed: stat?.missed ?? 0,
      timeout: stat?.timeout ?? 0,
      unviable: stat?.unviable ?? 0,
      score,
      threshold: spec.threshold,
      gap,
    });
    if (gap) {
      failures.push(`${name}: mutation score ${score.toFixed(1)}% (threshold ${spec.threshold}%)`);
    }
  }
  return { rows, failures, stats };
}

export function formatMutationTable(rows) {
  const header = ['crate', 'tested', 'caught', 'missed', 'timeout', 'score', 'threshold'];
  const lines = rows.map((row) => [
    row.crate,
    String(row.tested),
    String(row.caught),
    String(row.missed),
    String(row.timeout),
    row.present ? `${row.score.toFixed(1)}%` : 'absent',
    `${row.threshold}%`,
  ]);
  const widths = header.map((_, index) =>
    Math.max(header[index].length, ...lines.map((line) => line[index].length)),
  );
  const pad = (line) => header.map((_, index) => line[index].padEnd(widths[index])).join('  ');
  return [pad(header), lines.map(pad).join('\n')].join('\n');
}

function loadCargoMetadata() {
  const metadata = JSON.parse(
    execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--no-deps'], {
      encoding: 'utf8',
    }),
  );
  const members = new Set(metadata.workspace_members);
  return metadata.packages.filter((pkg) => members.has(pkg.id));
}

const isCli = process.argv[1]?.endsWith('check-mutation.mjs');
if (isCli) {
  const [outcomesPath, configPath = 'docs/testing/rust-mutation-thresholds.json'] =
    process.argv.slice(2);
  if (!outcomesPath)
    throw new Error('usage: check-mutation.mjs <cargo-mutants-outcomes.json> [thresholds.json]');
  const outcomes = JSON.parse(readFileSync(outcomesPath, 'utf8'));
  const config = JSON.parse(readFileSync(configPath, 'utf8'));
  if (!Array.isArray(outcomes.outcomes))
    throw new Error(
      `${outcomesPath} is not a cargo-mutants outcomes.json (expected an "outcomes" array)`,
    );
  const packages = loadCargoMetadata();
  for (const name of Object.keys(config.crates)) {
    const pkg = packages.find((entry) => entry.name === name);
    if (!pkg) throw new Error(`Configured crate ${name} is not a workspace member`);
    rustMutation({ name, ...rustSources(pkg.manifest_path, process.cwd()) }, outcomes);
  }
  const result = evaluateMutation(outcomes.outcomes, config, packages);
  console.log(formatMutationTable(result.rows));
  if (result.failures.length) {
    console.error(`error: ${result.failures.length} mutation score gap(s)`);
    for (const failure of result.failures) console.error(`  - ${failure}`);
    process.exit(1);
  }
}
