import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { typescriptUnits } from './v1-catalogue.mjs';
import { typescriptCoverage, typescriptMutation } from './v1-typescript-measurements.mjs';

export function typescriptScope(name, cwd = process.cwd()) {
  assert(/^[a-z][a-z0-9-]*$/u.test(name ?? ''), 'A TypeScript package slug is required');
  const root = JSON.parse(readFileSync(path.join(cwd, 'docs/testing/v1/manifest.json'), 'utf8'));
  const unit = typescriptUnits(root, cwd).find((entry) => entry.name === `@nvbes/${name}`);
  assert(unit, `Package outside applicable V1 scope: ${name}`);
  return { unit, thresholds: root.thresholds };
}

export function enforceTypescriptThresholds(unit, thresholds, scores) {
  for (const [metric, score] of Object.entries(scores)) {
    const configured = [thresholds[metric], unit.thresholds?.[metric] ?? 90];
    assert(
      configured.every((value) => Number.isFinite(value) && value >= 0 && value <= 100),
      `Invalid ${metric} threshold`,
    );
    const threshold = Math.max(90, ...configured);
    assert(
      Number.isFinite(score) && score >= threshold,
      `${unit.name}: ${metric} ${score}% < ${threshold}%`,
    );
  }
  return scores;
}

export function checkTypescriptReport(name, kind, report, cwd = process.cwd()) {
  const { unit, thresholds } = typescriptScope(name, cwd);
  assert(['coverage', 'mutation'].includes(kind), 'Unknown measurement kind');
  const scores =
    kind === 'coverage'
      ? typescriptCoverage(unit, report)
      : { mutation: typescriptMutation(unit, report) };
  return enforceTypescriptThresholds(unit, thresholds, scores);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const [name, kind, report] = process.argv.slice(2);
    assert.equal(process.argv.length, 5, 'usage: <package> <coverage|mutation> <report>');
    console.log(
      JSON.stringify(checkTypescriptReport(name, kind, JSON.parse(readFileSync(report, 'utf8')))),
    );
  } catch (error) {
    console.error(`TypeScript quality gate refused: ${error.message}`);
    process.exitCode = 1;
  }
}
