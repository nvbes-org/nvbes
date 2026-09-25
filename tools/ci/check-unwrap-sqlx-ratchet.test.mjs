import assert from 'node:assert/strict';
import test from 'node:test';
import { countUnwraps, isInsideCfgTest, isProductionRustPath } from './check-unwrap-ratchet.mjs';
import { countRuntimeSql } from './check-sqlx-ratchet.mjs';

test('isProductionRustPath excludes tests', () => {
  assert.equal(isProductionRustPath('libs/rust/core/src/auth.rs'), true);
  assert.equal(isProductionRustPath('libs/rust/core/src/auth.tests.rs'), false);
});

test('isInsideCfgTest detects test modules', () => {
  const source = [
    'fn prod() { x.unwrap(); }',
    '#[cfg(test)]',
    'mod tests {',
    '    #[test]',
    '    fn t() { y.unwrap(); }',
    '}',
  ].join('\n');
  assert.equal(isInsideCfgTest(source, 1), false);
  assert.equal(isInsideCfgTest(source, 5), true);
});

test('countUnwraps ignores cfg(test) and allow comments', () => {
  const source = [
    'fn a() { x.unwrap(); }',
    'fn b() { x.expect("e"); // nvbes-allow-unwrap }',
    '#[cfg(test)]',
    'mod tests {',
    '  fn t() { x.unwrap(); }',
    '}',
  ].join('\n');
  assert.equal(countUnwraps(source), 1);
});

test('countRuntimeSql ignores query! and tests', () => {
  const source = [
    'fn a() { sqlx::query("a"); }',
    'fn b() { sqlx::query!("b"); }',
    'fn c() { sqlx::query("c"); // nvbes-allow-runtime-sql }',
    '#[cfg(test)]',
    'mod tests { fn t() { sqlx::query("t"); } }',
  ].join('\n');
  assert.equal(countRuntimeSql(source), 1);
});
