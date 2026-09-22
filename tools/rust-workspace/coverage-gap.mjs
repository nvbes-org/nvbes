import { readFileSync } from 'node:fs';
import { crateDirectoryMapping, loadCargoMetadata, percentOf } from './check-coverage.mjs';

const IGNORE_FILENAME_REGEX = /(\.tests\.rs|\.test_support\.rs)$/u;

/**
 * Lignes couvertes supplementaires necessaires pour atteindre `target` pourcent.
 * Le denominateur est fixe : llvm-cov ne compte que les lignes instrumentees.
 */
export function missingToTarget([count, covered], target) {
  const required = Math.ceil((target * count) / 100);
  return Math.max(0, required - covered);
}

export function collectFileCoverage(report, packages) {
  const byDir = crateDirectoryMapping(packages);
  const prefixes = [...byDir.keys()].sort((a, b) => b.length - a.length);
  const files = [];
  for (const file of report.data.flatMap((entry) => entry.files ?? [])) {
    if (IGNORE_FILENAME_REGEX.test(file.filename)) continue;
    const dir = prefixes.find(
      (candidate) => file.filename.startsWith(`${candidate}/`) || file.filename === candidate,
    );
    if (!dir) continue;
    files.push({
      crate: byDir.get(dir),
      path: file.filename.slice(dir.lastIndexOf('/') + 1),
      lines: [file.summary.lines.count, file.summary.lines.covered],
      functions: [file.summary.functions.count, file.summary.functions.covered],
      regions: [file.summary.regions.count, file.summary.regions.covered],
    });
  }
  return files;
}

export function buildGapReport(report, packages, target, excluded = []) {
  const skipped = new Set(excluded);
  const files = collectFileCoverage(report, packages).filter((file) => !skipped.has(file.crate));
  const crates = new Map();
  for (const file of files) {
    if (!crates.has(file.crate))
      crates.set(file.crate, { crate: file.crate, lines: [0, 0], files: [] });
    const crate = crates.get(file.crate);
    crate.lines[0] += file.lines[0];
    crate.lines[1] += file.lines[1];
    crate.files.push(file);
  }
  const rows = [...crates.values()].map((crate) => ({
    crate: crate.crate,
    linesPct: percentOf(crate.lines),
    instrumented: crate.lines[0],
    uncovered: crate.lines[0] - crate.lines[1],
    missing: missingToTarget(crate.lines, target),
    files: crate.files
      .map((file) => ({
        path: file.path,
        linesPct: percentOf(file.lines),
        instrumented: file.lines[0],
        uncovered: file.lines[0] - file.lines[1],
      }))
      .filter((file) => file.uncovered > 0)
      .sort((a, b) => b.uncovered - a.uncovered),
  }));
  rows.sort((a, b) => b.missing - a.missing);
  const workspace = files.reduce(
    (totals, file) => [totals[0] + file.lines[0], totals[1] + file.lines[1]],
    [0, 0],
  );
  return {
    target,
    workspace: {
      linesPct: percentOf(workspace),
      instrumented: workspace[0],
      uncovered: workspace[0] - workspace[1],
      missing: missingToTarget(workspace, target),
    },
    crates: rows,
  };
}

export function formatGapReport(report, filesPerCrate) {
  const lines = [
    `workspace: ${report.workspace.linesPct.toFixed(1)}% lines ` +
      `(${report.workspace.uncovered} uncovered / ${report.workspace.instrumented} instrumented, ` +
      `${report.workspace.missing} lines to reach ${report.target}%)`,
    '',
  ];
  for (const crate of report.crates) {
    lines.push(
      `${crate.crate}: ${crate.linesPct.toFixed(1)}% ` +
        `(${crate.uncovered} uncovered, +${crate.missing} to ${report.target}%)`,
    );
    for (const file of crate.files.slice(0, filesPerCrate)) {
      lines.push(
        `    ${file.uncovered.toString().padStart(5)} uncovered  ` +
          `${file.linesPct.toFixed(0).padStart(3)}%  ${file.path}`,
      );
    }
    lines.push('');
  }
  return lines.join('\n');
}

const isCli = process.argv[1]?.endsWith('coverage-gap.mjs');
if (isCli) {
  const [
    reportPath = '.temp/rust/coverage-workspace.json',
    rawTarget = '90',
    rawFiles = '10',
    configPath = 'docs/testing/rust-coverage-thresholds.json',
  ] = process.argv.slice(2);
  const config = JSON.parse(readFileSync(configPath, 'utf8'));
  const report = buildGapReport(
    JSON.parse(readFileSync(reportPath, 'utf8')),
    loadCargoMetadata(),
    Number(rawTarget),
    config.excluded.map((entry) => entry.crate),
  );
  if (process.env.NVBES_COVERAGE_GAP_JSON) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    console.log(formatGapReport(report, Number(rawFiles)));
  }
}
