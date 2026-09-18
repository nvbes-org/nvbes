import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { confinedRead } from './v1-bundle-verification.mjs';
import { typescriptUnits } from './v1-catalogue.mjs';
import { enforceTypescriptThresholds } from './typescript-quality-gate.mjs';
import { loadV1 } from './v1-release.mjs';
import { ciContext } from './v1-suite-receipt.mjs';
import { typescriptMeasurements } from './v1-typescript-measurements.mjs';

const canonicalDate = (value) => new Date(value).toISOString();

export function typescriptMeasurementSlug(unit) {
  const match = /^@nvbes\/([a-z][a-z0-9-]*)$/u.exec(unit?.name ?? '');
  assert(match, 'Unsupported TypeScript measurement unit');
  return match[1];
}

export function produceTypescriptMeasurement({
  unit,
  context,
  coverageBytes,
  mutationBytes,
  completedAt,
  tools,
  thresholds,
}) {
  assert(unit?.language === 'typescript', 'TypeScript measurement unit required');
  assert(
    tools &&
      Object.keys(tools).length > 0 &&
      Object.values(tools).every((value) => typeof value === 'string' && value.trim()),
    'Tool versions required',
  );
  const coverage = JSON.parse(coverageBytes.toString('utf8'));
  const mutation = JSON.parse(mutationBytes.toString('utf8'));
  const scores = typescriptMeasurements(unit, coverage, mutation);
  enforceTypescriptThresholds(
    unit,
    thresholds ?? { lines: 90, branches: 90, mutation: 90 },
    scores,
  );
  return {
    measurementVersion: 1,
    language: 'typescript',
    unit: unit.name,
    sha: context.sha,
    completedAt: canonicalDate(completedAt),
    attempt: 1,
    tools,
    producer: context.producer,
    reports: { 'istanbul-summary': 'coverage.json', stryker: 'mutation.json' },
    artifacts: ['measurement.json', 'coverage.json', 'mutation.json'],
    ...scores,
  };
}

function argumentsByName(values) {
  assert(values.length === 3, 'usage: --unit=<name> --coverage=<path> --mutation=<path>');
  const options = new Map();
  for (const value of values) {
    const match = /^--(unit|coverage|mutation)=(.+)$/u.exec(value);
    assert(match && !options.has(match[1]), 'Invalid or duplicate measurement argument');
    options.set(match[1], match[2]);
  }
  assert(options.size === 3, 'Missing measurement argument');
  return options;
}

function toolVersions() {
  const packageVersion = (file) => JSON.parse(readFileSync(file, 'utf8')).version;
  const packageManager = JSON.parse(readFileSync('package.json', 'utf8')).packageManager;
  assert(/^pnpm@\d+\.\d+\.\d+$/u.test(packageManager), 'Pinned pnpm version required');
  return {
    node: process.version,
    pnpm: packageManager.slice('pnpm@'.length),
    vitest: packageVersion('node_modules/vitest/package.json'),
    stryker: packageVersion('node_modules/@stryker-mutator/core/package.json'),
  };
}

function main() {
  const options = argumentsByName(process.argv.slice(2));
  const cwd = process.cwd();
  const { root } = loadV1(cwd);
  const units = typescriptUnits(root, cwd);
  const unit = units.find((entry) => entry.name === options.get('unit'));
  assert(unit, 'Unknown candidate measurement unit');
  const context = ciContext(process.env);
  const coverageBytes = confinedRead(cwd, options.get('coverage'));
  const mutationBytes = confinedRead(cwd, options.get('mutation'));
  const measurement = produceTypescriptMeasurement({
    unit,
    context,
    coverageBytes,
    mutationBytes,
    completedAt: new Date(),
    tools: toolVersions(),
    thresholds: root.thresholds,
  });
  const output = path.join(
    cwd,
    '.temp/v1-measurements',
    `typescript-${typescriptMeasurementSlug(unit)}`,
  );
  mkdirSync(path.dirname(output), { recursive: true });
  mkdirSync(output);
  for (const [name, bytes] of [
    ['coverage.json', coverageBytes],
    ['mutation.json', mutationBytes],
  ]) {
    writeFileSync(path.join(output, name), bytes, { mode: 0o600, flag: 'wx' });
  }
  writeFileSync(
    path.join(output, 'measurement.json'),
    `${JSON.stringify(measurement, null, 2)}\n`,
    {
      mode: 0o600,
      flag: 'wx',
    },
  );
  console.log(`V1 measurement receipt written for ${measurement.unit}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    main();
  } catch (error) {
    console.error(`V1 measurement refused: ${error.message}`);
    process.exitCode = 1;
  }
}
