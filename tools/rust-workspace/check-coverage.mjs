import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const IGNORE_FILENAME_REGEX = /(\.tests\.rs|\.test_support\.rs)$/u;

export function crateDirectoryMapping(packages) {
  const byDir = new Map();
  for (const pkg of packages) {
    const dir = pkg.manifest_path.replace(/\/Cargo\.toml$/u, '');
    byDir.set(dir, pkg.name);
  }
  return byDir;
}

export function aggregateCrateCoverage(report, packages) {
  const byDir = crateDirectoryMapping(packages);
  const prefixes = [...byDir.keys()].sort((a, b) => b.length - a.length);
  const aggregated = new Map();
  const orphans = [];
  for (const file of report.data.flatMap((entry) => entry.files ?? [])) {
    if (IGNORE_FILENAME_REGEX.test(file.filename)) continue;
    let crate = null;
    for (const dir of prefixes) {
      if (file.filename.startsWith(`${dir}/`) || file.filename === dir) {
        crate = byDir.get(dir);
        break;
      }
    }
    if (!crate) {
      orphans.push(file.filename);
      continue;
    }
    if (!aggregated.has(crate)) {
      aggregated.set(crate, {
        lines: [0, 0],
        functions: [0, 0],
        regions: [0, 0],
        branches: [0, 0],
      });
    }
    const metric = aggregated.get(crate);
    metric.lines[0] += file.summary.lines.count;
    metric.lines[1] += file.summary.lines.covered;
    metric.functions[0] += file.summary.functions.count;
    metric.functions[1] += file.summary.functions.covered;
    metric.regions[0] += file.summary.regions.count;
    metric.regions[1] += file.summary.regions.covered;
    metric.branches[0] += file.summary.branches?.count ?? 0;
    metric.branches[1] += file.summary.branches?.covered ?? 0;
  }
  return { aggregated, orphans };
}

export function percentOf(aggregates) {
  const [count, covered] = aggregates;
  if (count === 0) return 0;
  return (covered * 100) / count;
}

export function evaluateCoverage(report, packages, config) {
  const { aggregated, orphans } = aggregateCrateCoverage(report, packages);
  if (config.schemaVersion !== 1)
    throw new Error(`Unsupported coverage schemaVersion ${config.schemaVersion}`);
  const memberNames = new Set(packages.map((pkg) => pkg.name));
  const configuredNames = new Set(Object.keys(config.crates));
  const excludedNames = new Set(config.excluded.map((entry) => entry.crate));
  for (const name of configuredNames) {
    if (!memberNames.has(name))
      throw new Error(`Configured crate ${name} is not a workspace member`);
    if (excludedNames.has(name)) throw new Error(`Crate ${name} is both configured and excluded`);
  }
  for (const entry of config.excluded) {
    if (!memberNames.has(entry.crate))
      throw new Error(`Excluded crate ${entry.crate} is not a workspace member`);
    if (!entry.reason) throw new Error(`Excluded crate ${entry.crate} requires an explicit reason`);
  }

  const rows = [];
  const failures = [];
  for (const [name, spec] of Object.entries(config.crates)) {
    const metric = aggregated.get(name);
    const linePct = metric ? percentOf(metric.lines) : 0;
    const functionPct = metric ? percentOf(metric.functions) : 0;
    const regionPct = metric ? percentOf(metric.regions) : 0;
    const lineGap = linePct < spec.threshold.lines;
    const functionGap = functionPct < spec.threshold.functions;
    const regionGap = regionPct < spec.threshold.regions;
    rows.push({
      crate: name,
      present: Boolean(metric),
      lines: linePct,
      functions: functionPct,
      regions: regionPct,
      threshold: spec.threshold,
      lineGap,
      functionGap,
      regionGap,
    });
    if (lineGap || functionGap || regionGap) {
      failures.push(
        `${name}: lines ${linePct.toFixed(1)}% (${spec.threshold.lines}%), ` +
          `functions ${functionPct.toFixed(1)}% (${spec.threshold.functions}%), ` +
          `regions ${regionPct.toFixed(1)}% (${spec.threshold.regions}%)`,
      );
    }
  }

  const uncovered = [...memberNames]
    .filter((name) => !configuredNames.has(name) && !excludedNames.has(name))
    .sort();
  for (const name of uncovered) {
    failures.push(`${name}: workspace member without configured threshold or explicit exclusion`);
  }
  return { rows, failures, orphans, uncovered };
}

export function formatCoverageTable(rows) {
  const header = ['crate', 'lines', 'functions', 'regions'];
  const lines = rows.map((row) => [
    row.crate,
    row.present ? `${row.lines.toFixed(1)}%` : 'absent',
    row.present ? `${row.functions.toFixed(1)}%` : 'absent',
    row.present ? `${row.regions.toFixed(1)}%` : 'absent',
  ]);
  const widths = header.map((_, index) =>
    Math.max(header[index].length, ...lines.map((line) => line[index].length)),
  );
  const pad = (line) => header.map((_, index) => line[index].padEnd(widths[index])).join('  ');
  return [pad(header), lines.map(pad).join('\n')].join('\n');
}

export function loadCargoMetadata() {
  const metadata = JSON.parse(
    execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--no-deps'], {
      encoding: 'utf8',
    }),
  );
  const members = new Set(metadata.workspace_members);
  return metadata.packages.filter((pkg) => members.has(pkg.id));
}

const isCli = process.argv[1]?.endsWith('check-coverage.mjs');
if (isCli) {
  const [reportPath, configPath = 'docs/testing/rust-coverage-thresholds.json'] =
    process.argv.slice(2);
  if (!reportPath) throw new Error('usage: check-coverage.mjs <llvm-cov-json> [thresholds.json]');
  const report = JSON.parse(readFileSync(reportPath, 'utf8'));
  const config = JSON.parse(readFileSync(configPath, 'utf8'));
  const packages = loadCargoMetadata();
  const result = evaluateCoverage(report, packages, config);
  console.log(formatCoverageTable(result.rows));
  if (result.uncovered.length) {
    console.error(`error: uncovered workspace members: ${result.uncovered.join(', ')}`);
  }
  if (result.orphans.length) {
    console.error(
      `error: ${result.orphans.length} report file(s) not attributed to a workspace crate`,
    );
  }
  if (result.failures.length) {
    console.error(`error: ${result.failures.length} coverage gap(s)`);
    for (const failure of result.failures) console.error(`  - ${failure}`);
    process.exit(1);
  }
}
