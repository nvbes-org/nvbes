import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { loadV1 } from '../test-summary/v1-context.mjs';
import { productionUnits } from '../test-summary/v1-catalogue.mjs';
import { rustMutation } from '../test-summary/v1-rust-measurements.mjs';

const context = loadV1(process.cwd());
const units = productionUnits(context.root, context.domains, process.cwd()).filter(
  (unit) => unit.language === 'rust',
);
assert(units.length > 0, 'Empty V1 mutation scope');
const args = process.argv.slice(2);
assert(
  args.length === 0 || (args.length === 1 && args[0] === '--list'),
  'usage: mutation-v1.mjs [--list]',
);
if (args[0] === '--list') {
  console.table(
    units.map((unit) => ({
      crate: unit.name,
      threshold: Math.max(90, unit.thresholds.mutation ?? 0),
    })),
  );
} else {
  assert.equal(
    execFileSync('cargo', ['mutants', '--version'], { encoding: 'utf8' }).trim(),
    'cargo-mutants 27.1.0',
    'Pinned cargo-mutants version required',
  );
  mkdirSync('.temp/rust-v1-mutation', { recursive: true });
  const directory = mkdtempSync('.temp/rust-v1-mutation/run-');
  const results = [];
  const measured = new Set();
  const run = {
    sha: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
    purpose: 'Local exhaustive mutation diagnostic; not authenticated release evidence.',
    results,
  };
  for (const [index, manifest] of context.root.cargoManifests.entries()) {
    const metadata = JSON.parse(
      execFileSync(
        'cargo',
        ['metadata', '--no-deps', '--locked', '--format-version', '1', '--manifest-path', manifest],
        { encoding: 'utf8' },
      ),
    );
    const members = metadata.packages.filter((pkg) => metadata.workspace_members.includes(pkg.id));
    const selected = units.filter((unit) => members.some((pkg) => pkg.name === unit.name));
    if (!selected.length) continue;
    const output = path.join(directory, String(index));
    const result = spawnSync(
      'cargo',
      [
        'mutants',
        '--manifest-path',
        manifest,
        ...selected.flatMap((unit) => ['--package', unit.name]),
        '--jobs',
        process.env.NVBES_MUTATION_JOBS ?? '2',
        '--timeout',
        process.env.NVBES_MUTATION_TIMEOUT ?? '600',
        '--no-times',
        '-o',
        output,
      ],
      { stdio: 'inherit' },
    );
    assert([0, 2, 3].includes(result.status), `Mutation tool failed: ${manifest}`);
    const report = JSON.parse(readFileSync(path.join(output, 'mutants.out/outcomes.json'), 'utf8'));
    for (const unit of selected) {
      assert(!measured.has(unit.name), `Duplicate mutation scope: ${unit.name}`);
      measured.add(unit.name);
      const score = rustMutation(unit, report);
      const threshold = Math.max(90, unit.thresholds.mutation ?? 0);
      results.push({ crate: unit.name, score, threshold, passed: score >= threshold });
    }
    writeFileSync(path.join(directory, 'summary.json'), `${JSON.stringify(run, null, 2)}\n`);
  }
  assert.equal(measured.size, units.length, 'Incomplete V1 mutation scope');
  console.table(results);
  console.log(`Mutation diagnostic: ${directory}/summary.json`);
  process.exitCode = results.every((row) => row.passed) ? 0 : 1;
}
