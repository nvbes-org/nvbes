import assert from 'node:assert/strict';
import test from 'node:test';
import { findWeakAssertionsInDiff } from './check-weak-assertions.mjs';

test('findWeakAssertionsInDiff catches production is_err / is_ok asserts', () => {
  const files = new Map([
    [
      'apps/x.rs',
      [
        'fn a() {',
        '    assert!(result.is_err());',
        '    assert!(ok.is_ok());',
        '    assert!(matches!(result, Err(Error::Gone)));',
        '    assert!(result.is_err()); // nvbes-allow-weak-assert',
        '}',
      ].join('\n'),
    ],
  ]);
  const diff = [
    'diff --git a/apps/x.rs b/apps/x.rs',
    '+++ b/apps/x.rs',
    '@@ -1,0 +1,5 @@',
    '+fn a() {',
    '+    assert!(result.is_err());',
    '+    assert!(ok.is_ok());',
    '+    assert!(matches!(result, Err(Error::Gone)));',
    '+    assert!(result.is_err()); // nvbes-allow-weak-assert',
  ].join('\n');
  const hits = findWeakAssertionsInDiff(diff, (path) => files.get(path) ?? null);
  assert.equal(hits.length, 2);
  assert.equal(hits[0].path, 'apps/x.rs');
});

test('ignores test files and context lines', () => {
  const diff = [
    '+++ b/tools/x.mjs',
    '+assert!(x.is_err());',
    '+++ b/apps/x.tests.rs',
    '@@ -1,0 +1,1 @@',
    '+    assert!(result.is_err());',
    '+++ b/apps/x.rs',
    '@@ -1,1 +1,1 @@',
    '     assert!(old.is_err());',
    '-    assert!(removed.is_ok());',
  ].join('\n');
  assert.equal(findWeakAssertionsInDiff(diff).length, 0);
});
