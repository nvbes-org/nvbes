import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const runner = readFileSync('tools/ci/run-lane.mjs', 'utf8');

test('contracts lane lints protobuf and checks compatibility against the planned base', () => {
  assert.match(runner, /run\('pnpm', \['check:contracts'\]\)/u);
  assert.match(runner, /'buf',\s*'breaking'/u);
  assert.match(runner, /`\.git#commit=\$\{plan\.base\},subdir=contracts\/protobuf`/u);
});
