import assert from 'node:assert/strict';
import test from 'node:test';

import { parseEnv, splitTemplateAssignment } from './env.mjs';

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

test('parses export prefixes and spacing around the assignment operator', () => {
  const env = parseEnv(
    [
      'PLAIN=1',
      'export PREFIXED=2',
      'exportTIGHT=3',
      'export=4',
      'SPACED   =   6',
      'TABBED\t=\t7',
      'EMPTY=',
    ].join('\n'),
  );

  assert.equal(env.assignments.get('PLAIN').rawValue, '1');
  assert.equal(env.assignments.get('PREFIXED').rawValue, '2');
  assert.equal(env.assignments.get('exportTIGHT').rawValue, '3');
  assert.equal(env.assignments.get('export').rawValue, '4');
  assert.equal(env.assignments.get('SPACED').rawValue, '6');
  assert.equal(env.assignments.get('TABBED').rawValue, '7');
  assert.equal(env.assignments.get('EMPTY').rawValue, '');

  // `export =5` is an assignment to the key literally named `export`.
  const exported = parseEnv('export =5\n');
  assert.equal(exported.assignments.get('export').rawValue, '5');
});

test('rejects assignments with dotted or spaced keys', () => {
  assert.throws(() => parseEnv('KEY.WITH.DOTS=1\n'), /expected KEY=value/);
  assert.throws(() => parseEnv('KEY WITH SPACE=1\n'), /expected KEY=value/);
  assert.throws(() => parseEnv('1NUMBERED=1\n'), /expected KEY=value/);
});

test('splits template assignments while preserving indentation and export', () => {
  assert.deepEqual(splitTemplateAssignment('KEY=value'), {
    key: 'KEY',
    prefix: '',
    separator: '=',
  });
  assert.deepEqual(splitTemplateAssignment('  export KEY = value'), {
    key: 'KEY',
    prefix: '  export ',
    separator: ' = ',
  });
  assert.deepEqual(splitTemplateAssignment('export =1'), {
    key: 'export',
    prefix: '',
    separator: ' =',
  });
  assert.equal(splitTemplateAssignment('# comment'), null);
  assert.equal(splitTemplateAssignment('  no assignment here'), null);
});
