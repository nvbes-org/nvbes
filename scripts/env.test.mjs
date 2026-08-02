import assert from 'node:assert/strict';
import test from 'node:test';

import { parseEnv } from './env.mjs';

test('parses dotenv values without evaluating shell syntax', () => {
  const env = parseEnv(
    [
      'PLAIN=value',
      'SPACED="hello world"',
      "LITERAL='$(touch /tmp/must-not-run)'",
      'COMMENTED=value # explanation',
    ].join('\n'),
  );

  assert.equal(env.assignments.get('PLAIN').value, 'value');
  assert.equal(env.assignments.get('SPACED').value, 'hello world');
  assert.equal(env.assignments.get('LITERAL').value, '$(touch /tmp/must-not-run)');
  assert.equal(env.assignments.get('COMMENTED').value, 'value');
});

test('rejects duplicate and executable dotenv lines', () => {
  assert.throws(() => parseEnv('KEY=first\nKEY=second\n'), /duplicate variables: KEY/);
  assert.throws(() => parseEnv('source another-file\n'), /expected KEY=value/);
});
