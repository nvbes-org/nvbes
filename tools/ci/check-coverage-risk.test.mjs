import test from 'node:test';
import assert from 'node:assert/strict';
import {
  isExcluded,
  parseLcovCoverage,
  auditChangedFilesForCoverage,
} from './check-coverage-risk.mjs';

test('isExcluded matches wildcard exclusions correctly', () => {
  const exclusions = [
    'archive/**',
    'tools/**',
    'libs/ts/identity-sdk-web/src/bot-guard*',
    'apps/identity-web/dev.routing.ts',
  ];

  assert.equal(isExcluded('archive/old.ts', exclusions), true);
  assert.equal(isExcluded('libs/ts/identity-sdk-web/src/bot-guard.decoy.ts', exclusions), true);
  assert.equal(isExcluded('libs/ts/identity-sdk-web/src/bot-guard.signals.ts', exclusions), true);
  assert.equal(isExcluded('apps/identity-web/dev.routing.ts', exclusions), true);
  assert.equal(isExcluded('apps/identity-service/src/service.rs', exclusions), false);
});

test('parseLcovCoverage accurately computes percentage', () => {
  const lcov = `
TN:
SF:apps/test/file.ts
DA:1,1
DA:2,1
DA:3,0
DA:4,1
LF:4
LH:3
end_of_record
`;
  const result = parseLcovCoverage(lcov);
  assert.notEqual(result, null);
  assert.equal(result.found, 4);
  assert.equal(result.hit, 3);
  assert.equal(result.percentage, 75.0);
});

test('auditChangedFilesForCoverage detects untested production files and respects exclusions', () => {
  const exclusions = ['libs/ts/identity-sdk-web/src/bot-guard*'];
  const changedFiles = [
    'libs/ts/identity-sdk-web/src/bot-guard.decoy.ts', // excluded -> safe
    'apps/identity-service/src/brand_new_untested.rs', // prod file without test
    'docs/readme.md', // not prod code -> ignored
  ];

  const audit = auditChangedFilesForCoverage(changedFiles, exclusions);
  assert.equal(
    audit.untestedFiles.includes('apps/identity-service/src/brand_new_untested.rs'),
    true,
  );
  assert.equal(
    audit.untestedFiles.includes('libs/ts/identity-sdk-web/src/bot-guard.decoy.ts'),
    false,
  );
});
