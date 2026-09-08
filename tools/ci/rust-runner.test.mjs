import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';
import test from 'node:test';

for (const failBaseline of [false, true]) {
  test(`Rust runner preserves full baseline (baseline failure=${failBaseline})`, () => {
    const directory = mkdtempSync(join(tmpdir(), 'nvbes-rust-runner-'));
    try {
      const metadata = {
        workspace_members: ['a', 'b'],
        packages: ['a', 'b'].map((name) => ({
          id: name,
          name,
          manifest_path: join(process.cwd(), 'apps', name, 'Cargo.toml'),
          dependencies: [],
        })),
      };
      const cargo = `#!${process.execPath}\nconst fs=require('node:fs');const args=process.argv.slice(2);fs.appendFileSync(process.env.TEST_CALLS,JSON.stringify(args)+'\\n');if(args[0]==='metadata') console.log(process.env.TEST_METADATA);if(args[0]==='test'&&args.includes('--workspace')&&process.env.TEST_FAIL==='true')process.exit(1);`;
      writeFileSync(join(directory, 'cargo'), cargo, { mode: 0o755 });
      writeFileSync(join(directory, 'event.json'), '{}');
      const result = spawnSync(process.execPath, ['tools/ci/run-rust.mjs'], {
        encoding: 'utf8',
        env: {
          ...process.env,
          PATH: `${directory}${delimiter}${process.env.PATH}`,
          TEST_CALLS: join(directory, 'calls'),
          TEST_METADATA: JSON.stringify(metadata),
          TEST_FAIL: String(failBaseline),
          GITHUB_STEP_SUMMARY: join(directory, 'summary'),
          GITHUB_EVENT_PATH: join(directory, 'event.json'),
          NVBES_CI_PLAN: JSON.stringify({
            version: 1,
            candidate: { rust: true },
            rustMode: 'workspace',
            paths: ['apps/a/src/main.rs'],
            head: 'a'.repeat(40),
            base: 'b'.repeat(40),
          }),
        },
      });
      assert.equal(result.status, failBaseline ? 1 : 0, result.stderr);
      const calls = readFileSync(join(directory, 'calls'), 'utf8')
        .trim()
        .split('\n')
        .map(JSON.parse);
      assert.deepEqual(
        calls.filter((args) => args[0] === 'test'),
        [
          ['test', '--locked', '--package', 'a'],
          ['test', '--workspace', '--locked'],
        ],
      );
      const evidence = JSON.parse(result.stdout.match(/CI_RUST_EVIDENCE (.+)/u)[1]);
      assert.equal(evidence.divergence, failBaseline);
      assert.equal(evidence.fullSuccess, !failBaseline);
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  });
}
