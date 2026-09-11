import assert from 'node:assert/strict';
import { test } from 'node:test';
import {
  assembleReport,
  buildCategoryRows,
  activeLanes,
  formatCategories,
  gapCategories,
  planReleaseDecision,
  statusMark,
  sortCategories,
} from './generate-test-summary-report.mjs';

function sampleManifest() {
  return {
    schemaVersion: 4,
    scope: { applications: 'account-service', libraries: 'http-client' },
    lanes: {
      pullRequest: { mode: 'automated' },
      preRelease: { mode: 'human' },
      gap: { mode: 'gap' },
    },
    coverage: [
      { category: 'unit', lane: 'pullRequest', artifact: 'Nx test reports' },
      { category: 'security', lane: 'pullRequest', artifact: 'check:security output' },
    ],
    knownGaps: [
      {
        categories: ['security'],
        description: 'DAST fixed-credential limitation.',
        releaseBlocking: true,
      },
    ],
    releaseReadiness: {
      blockingCategories: ['security'],
      status: 'blocked',
    },
  };
}

function sampleCoverage() {
  return {
    rows: [
      {
        crate: 'nvbes-core',
        present: true,
        lines: 43,
        functions: 43,
        regions: 43,
      },
    ],
    failures: [],
    orphans: [],
    uncovered: [],
    totalCrates: 1,
  };
}

test('statusMark maps booleans to glyphs', () => {
  assert.equal(statusMark(true), '✅');
  assert.equal(statusMark(false), '❌');
});

test('formatCategories and sortCategories order deterministically', () => {
  assert.equal(formatCategories(['b', 'a']), 'b, a');
  assert.deepEqual(sortCategories(new Set(['b', 'a', 'c'])), ['a', 'b', 'c']);
});

test('planReleaseDecision derives NO-GO with blockers', () => {
  assert.equal(planReleaseDecision([]).verdict, 'NO-GO');
  const decision = planReleaseDecision(['security', 'fuzz']);
  assert.equal(decision.verdict, 'NO-GO');
  assert.match(decision.reason, /2 release-blocking gap\(s\)/u);
});

test('gapCategories collects known-gap categories', () => {
  const gaps = gapCategories(sampleManifest());
  assert.deepEqual([...gaps], ['security']);
});

test('activeLanes excludes gap lanes', () => {
  assert.deepEqual(activeLanes(sampleManifest()), ['preRelease', 'pullRequest']);
});

test('buildCategoryRows marks gapped categories failed', () => {
  const rows = buildCategoryRows(sampleManifest(), gapCategories(sampleManifest()));
  const byCategory = Object.fromEntries(rows.map((row) => [row.category, row]));
  assert.equal(byCategory.unit.status, 'not-run');
  assert.equal(byCategory.security.status, '❌');
  assert.equal(byCategory.unit.lane, 'pullRequest');
});

test('assembleReport renders the 8 TSR sections with NO-GO', () => {
  const markdown = assembleReport({
    manifest: sampleManifest(),
    coverage: sampleCoverage(),
    meta: {
      release: '0.1.0-diag',
      generatedAt: new Date('2026-01-01T00:00:00.000Z'),
      coverageBaselineCommit: '6af12ea1',
      coverageBaselineDate: '2026-09-09',
    },
  });
  assert.match(markdown, /^# Test Summary Report — Release 0\.1\.0-diag/u);
  for (const section of [
    '## 1. Résumé exécutif',
    '## 2. Portée',
    '## 3. Résultats par catégorie',
    '## 4. Couverture',
    '## 5. Défauts',
    '## 6. Risques résiduels',
    '## 7. Recommandations',
    '## 8. Approbations',
  ]) {
    assert.ok(markdown.includes(section), `missing ${section}`);
  }
  assert.match(markdown, /\| security \| pullRequest \| ❌ \|/u);
  assert.match(markdown, /\*\*Verdict global\*\*: NO-GO/u);
  assert.match(markdown, /Go \/ No-go : no-go/u);
});

test('legacy report cannot render GO merely because there are no declared blockers', () => {
  const manifest = sampleManifest();
  manifest.knownGaps = [];
  manifest.releaseReadiness = { blockingCategories: [], status: 'ready' };
  const markdown = assembleReport({
    manifest,
    coverage: sampleCoverage(),
    meta: {
      release: '0.1.0',
      generatedAt: new Date('2026-01-01T00:00:00.000Z'),
      coverageBaselineCommit: '6af12ea1',
      coverageBaselineDate: '2026-09-09',
    },
  });
  assert.match(markdown, /\*\*Verdict global\*\*: NO-GO/u);
  assert.match(markdown, /Go \/ No-go : no-go/u);
  assert.match(markdown, /## 5\. Défauts\n\nAucun écart connu\./u);
});

test('assembleReport surfaces coverage gaps in section 4', () => {
  const coverage = sampleCoverage();
  coverage.failures = ['nvbes-core: lines 43.0% (95%)'];
  const markdown = assembleReport({
    manifest: sampleManifest(),
    coverage,
    meta: {
      release: '0.1.0',
      generatedAt: new Date('2026-01-01T00:00:00.000Z'),
      coverageBaselineCommit: '6af12ea1',
      coverageBaselineDate: '2026-09-09',
    },
  });
  assert.match(markdown, /## 4\. Couverture\n\n❌ 1 crate\(s\) sous seuil/u);
});
