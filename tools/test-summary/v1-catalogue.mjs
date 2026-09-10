import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync, readdirSync } from 'node:fs';
import path from 'node:path';
import ts from 'typescript';

export function typescriptSources(location, relativeRoot) {
  const sources = {};
  for (const file of readdirSync(path.join(location, 'src'), { recursive: true })) {
    if (!/\.(ts|tsx)$/u.test(file) || /\.(gen|test|spec|d)\.(ts|tsx)$/u.test(file)) continue;
    const source = readFileSync(path.join(location, 'src', file), 'utf8');
    const emitted = ts
      .transpileModule(source, {
        compilerOptions: { module: ts.ModuleKind.ESNext, removeComments: true },
        fileName: file,
      })
      .outputText.trim();
    sources[`${relativeRoot}/src/${file.replaceAll(path.sep, '/')}`] = {
      sha256: createHash('sha256').update(source).digest('hex'),
      runtime: emitted !== '' && emitted !== 'export {};',
    };
  }
  return sources;
}

export function cargoClosure(packages, seeds) {
  const byName = new Map(packages.map((pkg) => [pkg.name, pkg]));
  const visited = new Set();
  function visit(name) {
    if (visited.has(name)) return;
    const pkg = byName.get(name);
    assert(pkg, `Missing Cargo metadata: ${name}`);
    visited.add(name);
    for (const dep of pkg.dependencies) {
      if (dep.path && dep.kind !== 'dev') visit(dep.name);
    }
  }
  for (const seed of seeds) visit(seed);
  return [...visited].sort((a, b) => a.localeCompare(b));
}

export function typescriptThresholds(pkg) {
  const thresholds = {};
  const scripts = Object.values(pkg.scripts ?? {}).join(' ');
  for (const metric of ['lines', 'branches']) {
    const pattern = new RegExp(
      `--coverage\\.thresholds\\.${metric}=(\\d+(?:\\.\\d+)?)(?=\\s|$)`,
      'gu',
    );
    const values = [...scripts.matchAll(pattern)].map((match) => Number(match[1]));
    if (values.length) thresholds[metric] = Math.max(...values);
  }
  return thresholds;
}

export function productionUnits(root, domains, cwd) {
  const packages = root.cargoManifests.flatMap(
    (manifest) =>
      JSON.parse(
        execFileSync(
          'cargo',
          [
            'metadata',
            '--locked',
            '--no-deps',
            '--format-version',
            '1',
            '--manifest-path',
            manifest,
          ],
          { cwd, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 },
        ),
      ).packages,
  );
  const thresholds = {};
  for (const [file, metric] of [
    ['rust-coverage-thresholds.json', 'lines'],
    ['rust-condition-thresholds.json', 'branches'],
    ['rust-mutation-thresholds.json', 'mutation'],
  ]) {
    const config = JSON.parse(readFileSync(path.join(cwd, 'docs/testing', file), 'utf8'));
    for (const [name, spec] of Object.entries(config.crates)) {
      thresholds[name] ??= {};
      thresholds[name][metric] = metric === 'lines' ? spec.threshold.lines : spec.threshold;
    }
  }
  const names = cargoClosure(packages, [
    ...domains.map((domain) => domain.cargoPackage),
    ...root.sharedCargoPackages,
  ]);
  const units = names.map((name) => ({
    name,
    language: 'rust',
    thresholds: thresholds[name] ?? {},
  }));
  const manifests = execFileSync('rg', ['--files', 'libs/ts', '-g', 'package.json'], {
    cwd,
    encoding: 'utf8',
  })
    .trim()
    .split('\n');
  for (const manifest of manifests) {
    const location = path.dirname(path.join(cwd, manifest));
    const pkg = JSON.parse(readFileSync(path.join(location, 'package.json'), 'utf8'));
    if (Object.hasOwn(root.typescriptExclusions, pkg.name)) {
      const files = readdirSync(location, { recursive: true })
        .filter(
          (file) =>
            !file.startsWith('node_modules/') &&
            !file.startsWith('dist/') &&
            /\.(ts|tsx)$/u.test(file) &&
            !/\.(gen|test|spec|d)\.ts$/u.test(file),
        )
        .filter((file) => {
          const emitted = ts
            .transpileModule(readFileSync(path.join(location, file), 'utf8'), {
              compilerOptions: { module: ts.ModuleKind.ESNext, removeComments: true },
              fileName: file,
            })
            .outputText.trim();
          return emitted !== '' && emitted !== 'export {};';
        });
      assert.equal(
        files.length,
        0,
        `${pkg.name}: excluded package contains handwritten runtime TypeScript`,
      );
    } else
      units.push({
        name: pkg.name,
        language: 'typescript',
        thresholds: typescriptThresholds(pkg),
        sourceFiles: typescriptSources(location, path.dirname(manifest)),
      });
  }
  return units.sort((a, b) => a.name.localeCompare(b.name));
}
