import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';
import test from 'node:test';

for (const [failBaseline, failMicro, failClippy] of [
  [false, false, false],
  [true, false, false],
  [false, true, false],
  [false, false, true],
]) {
  test(`Rust runner preserves gates (baseline=${failBaseline}, micro=${failMicro}, clippy=${failClippy})`, () => {
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
      const cargo = `#!${process.execPath}\nconst fs=require('node:fs');const args=process.argv.slice(2);fs.appendFileSync(process.env.TEST_CALLS,JSON.stringify(args)+'\\n');if(args[0]==='metadata') console.log(process.env.TEST_METADATA);if(args[0]==='nextest'&&args[1]==='run'&&args.includes('--workspace')&&process.env.TEST_FAIL==='true')process.exit(1);if(args[0]==='clippy'&&process.env.TEST_CLIPPY_FAIL==='true')process.exit(1);`;
      writeFileSync(join(directory, 'cargo'), cargo, { mode: 0o755 });
      writeFileSync(
        join(directory, 'pnpm'),
        `#!${process.execPath}\nprocess.exit(${failMicro ? 1 : 0});`,
        {
          mode: 0o755,
        },
      );
      writeFileSync(join(directory, 'event.json'), '{}');
      const result = spawnSync(process.execPath, ['tools/ci/run-rust.mjs'], {
        encoding: 'utf8',
        env: {
          ...process.env,
          PATH: `${directory}${delimiter}${process.env.PATH}`,
          TEST_CALLS: join(directory, 'calls'),
          TEST_METADATA: JSON.stringify(metadata),
          TEST_FAIL: String(failBaseline),
          TEST_CLIPPY_FAIL: String(failClippy),
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
      assert.equal(result.status, failBaseline || failMicro || failClippy ? 1 : 0, result.stderr);
      const calls = readFileSync(join(directory, 'calls'), 'utf8')
        .trim()
        .split('\n')
        .map(JSON.parse);
      assert.ok(
        calls.some((args) =>
          args.join(' ').includes('clippy --workspace --all-targets --locked -- -D warnings'),
        ),
      );
      if (failMicro || failClippy) {
        assert.match(
          result.stderr,
          failClippy ? /Rust linting failed/u : /Isolated micro-tests failed/u,
        );
        assert.equal(
          calls.some((args) => args[0] === 'nextest'),
          false,
        );
        assert.doesNotMatch(result.stdout, /CI_RUST_EVIDENCE/u);
        return;
      }
      assert.deepEqual(
        calls.filter((args) => args[0] === 'nextest'),
        [
          ['nextest', 'run', '--locked', '--no-fail-fast', '--package', 'a'],
          ['nextest', 'run', '--workspace', '--locked', '--no-fail-fast'],
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
