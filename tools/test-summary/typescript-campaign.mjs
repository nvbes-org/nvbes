import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { pathToFileURL } from 'node:url';
import { typescriptUnits } from './v1-catalogue.mjs';

export async function runTypescriptCampaign(
  units,
  { coverageOnly = false, run, checkpoint = () => {} },
) {
  assert(units.length > 0, 'Empty TypeScript campaign');
  assert.equal(
    new Set(units.map((unit) => unit.name)).size,
    units.length,
    'Duplicate campaign unit',
  );
  const results = units.map((unit) => ({
    unit: unit.name,
    coverage: 'not-run',
    mutation: 'not-run',
  }));
  await checkpoint(results);
  for (const result of results) {
    for (const kind of coverageOnly ? ['coverage'] : ['coverage', 'mutation']) {
      try {
        result[kind] = (await run(result.unit, kind)) === 0 ? 'passed' : 'failed';
      } catch {
        result[kind] = 'blocked';
      }
      await checkpoint(results);
    }
  }
  return {
    results,
    passed: results.every((row) => row.coverage === 'passed' && row.mutation === 'passed'),
  };
}

async function main() {
  const args = process.argv.slice(2);
  assert(
    args.length === 0 || (args.length === 1 && args[0] === '--coverage-only'),
    'usage: [--coverage-only]',
  );
  const cwd = process.cwd();
  const manifest = JSON.parse(readFileSync('docs/testing/v1/manifest.json', 'utf8'));
  const units = typescriptUnits(manifest, cwd);
  const startedAt = new Date().toISOString();
  const output = path.join(cwd, '.temp/typescript-campaign', startedAt.replaceAll(':', '-'));
  mkdirSync(output, { recursive: true });
  const context = {
    schemaVersion: 1,
    sha: execFileSync('git', ['rev-parse', 'HEAD'], { encoding: 'utf8' }).trim(),
    trackedChanges:
      execFileSync('git', ['status', '--porcelain', '--untracked-files=no'], {
        encoding: 'utf8',
      }).trim() !== '',
    startedAt,
    releaseDecision: 'NO-GO',
    purpose:
      'Local diagnostic campaign; authenticated release evidence must be collected separately.',
    sources: units.map(({ name, sourceFiles }) => ({ name, sourceFiles })),
  };
  const campaign = await runTypescriptCampaign(units, {
    coverageOnly: args.includes('--coverage-only'),
    run: async (name, kind) => {
      const slug = name.slice('@nvbes/'.length);
      assert(/^[a-z][a-z0-9-]*$/u.test(slug), 'Unsafe package name');
      console.log(`Measuring ${name}: ${kind}`);
      const result = spawnSync(
        './node_modules/.bin/nx',
        ['run', `test-summary:${kind}:typescript`, '--skip-nx-cache'],
        {
          cwd,
          env: { ...process.env, [`NVBES_${kind.toUpperCase()}_PACKAGE`]: slug },
          stdio: 'inherit',
        },
      );
      if (result.error || result.signal) throw new Error('Campaign process interrupted');
      return result.status;
    },
    checkpoint: (results) =>
      writeFileSync(
        path.join(output, 'summary.json'),
        `${JSON.stringify({ ...context, updatedAt: new Date().toISOString(), results }, null, 2)}\n`,
      ),
  });
  console.table(campaign.results);
  console.log(`Diagnostic summary: ${path.relative(cwd, output)}/summary.json`);
  process.exitCode = campaign.passed ? 0 : 1;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
