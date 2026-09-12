import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { performance } from 'node:perf_hooks';
import { networkSandbox, isolatedRun, verifySandbox } from './micro-test.sandbox.mjs';

// All ordinary lib/bin tests are included unless explicitly classified as components.
// Default cargo test and the existing CI component/database lanes retain those tests.
const components = JSON.parse(
  readFileSync('tools/rust-workspace/micro-test.components.json', 'utf8'),
);
for (const [pkg, entries] of Object.entries(components)) {
  for (const [prefix, reason] of Object.entries(entries)) {
    assert(
      prefix.length > 0 && typeof reason === 'string' && reason.trim().length > 0,
      `Component classification requires a prefix and reason: ${pkg}`,
    );
  }
}
const manifests = JSON.parse(readFileSync('docs/testing/v1/manifest.json', 'utf8')).cargoManifests;
const binaries = [];
for (const manifest of manifests) {
  const metadata = JSON.parse(
    execFileSync(
      'cargo',
      ['metadata', '--no-deps', '--locked', '--format-version', '1', '--manifest-path', manifest],
      { encoding: 'utf8' },
    ),
  );
  const hasLibrary = metadata.packages.some(
    (pkg) =>
      metadata.workspace_members.includes(pkg.id) &&
      pkg.targets.some((target) => target.kind.includes('lib')),
  );
  const build = spawnSync(
    'cargo',
    [
      'test',
      '--workspace',
      ...(hasLibrary ? ['--lib'] : []),
      '--bins',
      '--locked',
      '--manifest-path',
      manifest,
      '--no-run',
      '--message-format=json',
    ],
    {
      encoding: 'utf8',
      maxBuffer: 32 * 1024 * 1024,
      stdio: ['ignore', 'pipe', 'inherit'],
    },
  );
  assert.equal(build.status, 0, `Micro-test compilation failed: ${manifest}`);
  binaries.push(
    ...build.stdout
      .trim()
      .split('\n')
      .map(JSON.parse)
      .filter(
        (entry) => entry.reason === 'compiler-artifact' && entry.profile.test && entry.executable,
      ),
  );
}
assert(binaries.length > 0, 'Empty Rust micro-test inventory');
const sandbox = networkSandbox();
const env = Object.fromEntries(
  ['PATH', 'TMPDIR', 'DYLD_LIBRARY_PATH', 'LD_LIBRARY_PATH']
    .filter((key) => process.env[key])
    .map((key) => [key, process.env[key]]),
);
Object.assign(env, {
  TZ: 'UTC',
  LANG: 'C',
  LC_ALL: 'C',
  PROPTEST_RNG_SEED: '20260912',
  PROPTEST_CASES: '2048',
  CARGO_MANIFEST_DIR: process.cwd(),
});
let count = 0;
let separated = 0;
const seen = new Set();
const started = performance.now();
try {
  verifySandbox(sandbox);
  for (const binary of binaries) {
    const pkg = binary.package_id.split('#').at(-1).split('@')[0];
    const listing = isolatedRun(sandbox, binary.executable, ['--list', '--format=terse'], { env });
    assert.equal(listing.status, 0, `${pkg}: cannot enumerate tests: ${listing.stderr}`);
    const names = listing.stdout
      .split('\n')
      .filter((line) => line.endsWith(': test'))
      .map((line) => line.slice(0, -6));
    const prefixes = Object.keys(components[pkg] ?? {});
    const excluded = names.filter((name) => prefixes.some((prefix) => name.startsWith(prefix)));
    for (const prefix of prefixes) {
      if (names.some((name) => name.startsWith(prefix))) seen.add(`${pkg}/${prefix}`);
    }
    if (names.length === excluded.length) {
      separated += excluded.length;
      console.log(`${pkg}: ${excluded.length} component tests, no micro-tests`);
      continue;
    }
    const result = isolatedRun(
      sandbox,
      binary.executable,
      ['--test-threads=1', '--format=terse', ...excluded.flatMap((name) => ['--skip', name])],
      { env: { ...env, CARGO_MANIFEST_DIR: binary.manifest_path.replace(/\/Cargo.toml$/u, '') } },
    );
    console.log(`${pkg}:\n${result.stdout}`);
    assert.equal(
      result.status,
      0,
      `${pkg}: isolated tests failed (${result.signal ?? result.error ?? ''})\n${result.stderr}`,
    );
    count += names.length - excluded.length;
    separated += excluded.length;
  }
  for (const [pkg, entries] of Object.entries(components)) {
    for (const prefix of Object.keys(entries))
      assert(seen.has(`${pkg}/${prefix}`), `Stale component classification: ${pkg}/${prefix}`);
  }
  const ts = isolatedRun(
    sandbox,
    process.execPath,
    [
      'node_modules/vite-plus/bin/vp',
      'test',
      'run',
      '--root',
      'libs/ts/email-ui',
      '--retry=0',
      '--maxWorkers=1',
    ],
    { env, stdio: 'inherit' },
  );
  assert.equal(ts.status, 0, 'Isolated email-ui tests failed');
  const contracts = isolatedRun(
    sandbox,
    process.execPath,
    ['--test', 'libs/ts/identity-sdk-core/openapi.contract.test.mjs'],
    { env, stdio: 'inherit' },
  );
  assert.equal(contracts.status, 0, 'Isolated generated SDK contract tests failed');
  console.log(
    `Micro-tests: ${count} Rust cases inventoried, ${separated} component cases separated; execution ${(performance.now() - started).toFixed(0)} ms (compilation excluded).`,
  );
} finally {
  sandbox.cleanup();
}
