#!/usr/bin/env node
import { evaluateCoverage, formatCoverageTable } from '../rust-workspace/check-coverage.mjs';
import { deriveReleaseBlockingCategories } from '../account-quality/check-account-release-readiness.mjs';
import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';

const STATUS_PASS = '✅';
const STATUS_FAIL = '❌';

export function statusMark(passed) {
  return passed ? STATUS_PASS : STATUS_FAIL;
}

export function formatCategories(categories) {
  return categories.join(', ');
}

export function sortCategories(categories) {
  return [...categories].sort((a, b) => a.localeCompare(b, 'en'));
}

export function planReleaseDecision(blockers, coverageFailures = []) {
  return {
    verdict: 'NO-GO',
    reason: `${blockers.length} release-blocking gap(s), ${coverageFailures.length} coverage failure(s); legacy diagnostic report has no verified candidate evidence; use test-summary:release`,
  };
}

export function gapCategories(manifest) {
  return new Set((manifest.knownGaps ?? []).flatMap((gap) => gap.categories ?? []));
}

export function activeLanes(manifest) {
  return sortCategories(
    Object.entries(manifest.lanes ?? {})
      .filter(([, lane]) => lane?.mode !== 'gap')
      .map(([name]) => name),
  );
}

export function buildCategoryRows(manifest, gaps) {
  return (manifest.coverage ?? [])
    .map((contract) => ({
      category: contract.category,
      lane: contract.lane,
      status: gaps.has(contract.category) ? statusMark(false) : 'not-run',
      evidence: contract.artifact ?? contract.blocker ?? 'n/a',
    }))
    .sort((a, b) => a.category.localeCompare(b.category, 'en'));
}

export function safeCoverageConfig(thresholds) {
  return {
    schemaVersion: thresholds.schemaVersion,
    crates: thresholds.crates,
    excluded: thresholds.excluded,
  };
}

function inlinePrefixed(text) {
  const fence = '```';
  return `${fence}\n${text}\n${fence}`;
}

function renderHeadline(meta) {
  return [
    `# Test Summary Report — Release ${meta.release}`,
    '',
    '## 1. Résumé exécutif',
    `- **Version testée**: \`${meta.release}\``,
    `- **Date de génération**: ${meta.generatedAt.toISOString()}`,
    `- **Baseline couverture**: \`${meta.coverageBaselineCommit}\` (` +
      `${meta.coverageBaselineDate})`,
    `- **Verdict global**: ${meta.verdict} — ${meta.verdictReason}`,
    '',
  ].join('\n');
}

function renderScope(manifest) {
  const scopeLines = Object.entries(manifest.scope ?? {}).map(
    ([key, value]) => `- **${key}**: ${value}`,
  );
  const lanes = activeLanes(manifest);
  return [
    '## 2. Portée',
    `Manifeste: \`docs/testing/account-test-manifest.json\` ` +
      `(schemaVersion ${manifest.schemaVersion}).`,
    ...scopeLines,
    `- **Lanes actives**: ${formatCategories(lanes)}`,
    '',
  ].join('\n');
}

function renderCategories(rows) {
  if (rows.length === 0) return '';
  const table = ['| Catégorie | Lane | Statut | Preuve |', '|---|---|---|---|'];
  for (const row of rows) {
    table.push(`| ${row.category} | ${row.lane} | ${row.status} | ${row.evidence} |`);
  }
  return ['## 3. Résultats par catégorie', ...table, ''].join('\n');
}

function renderCoverage(coverage) {
  const headline = coverage.failures.length
    ? `${STATUS_FAIL} ${coverage.failures.length} crate(s) sous seuil`
    : `${STATUS_PASS} couverture au-dessus des seuils`;
  const orphanNote = coverage.orphans.length
    ? `> ${coverage.orphans.length} fichier(s) de couverture non rattachés à une crate ` +
      `(${formatCategories(coverage.orphans)}).`
    : '';
  return [
    '## 4. Couverture',
    '',
    headline,
    `Thresholds: \`docs/testing/rust-coverage-thresholds.json\` ` +
      `(${coverage.totalCrates} crates surveillées).`,
    inlinePrefixed(formatCoverageTable(coverage.rows)),
    ...(orphanNote ? [orphanNote] : []),
    '',
  ].join('\n');
}

function renderDefects(manifest) {
  const knownGaps = manifest.knownGaps ?? [];
  if (knownGaps.length === 0) return '## 5. Défauts\n\nAucun écart connu.\n';
  const lines = knownGaps.map(
    (gap) =>
      `- **[${STATUS_FAIL}] ${formatCategories(gap.categories ?? [])}**: ` + `${gap.description}`,
  );
  return ['## 5. Défauts', ...lines, ''].join('\n');
}

function renderRisks(manifest) {
  const blockers = deriveReleaseBlockingCategories(manifest);
  if (blockers.length === 0) {
    return '## 6. Risques résiduels\n\nAucun risque résiduel bloquant.\n';
  }
  const gapLines = (manifest.knownGaps ?? [])
    .filter((gap) => gap.releaseBlocking === true)
    .map(
      (gap) =>
        `- **${formatCategories(gap.categories)}**: ${gap.description} ` + `(releaseBlocking=true)`,
    );
  return [
    '## 6. Risques résiduels',
    `Catégories bloquantes (${blockers.length}): ${formatCategories(blockers)}.`,
    ...gapLines,
    '',
  ].join('\n');
}

function renderRecommendations(verdict) {
  const lines =
    verdict === 'GO'
      ? [
          '- Go / No-go : go.',
          '- Idéalement traiter les écarts non-bloquants avant la release finale.',
        ]
      : [
          '- Go / No-go : no-go.',
          '- Traiter les risques résiduels et relancer la génération du rapport.',
        ];
  return ['## 7. Recommandations', ...lines, ''].join('\n');
}

function renderApprovals() {
  return [
    '## 8. Approbations',
    '| Rôle | Nom | Date | Signature |',
    '|------|-----|------|-----------|',
    '| Test Manager | ... | ... | ... |',
    '| Security Owner | ... | ... | ... |',
    '| Product Owner | ... | ... | ... |',
    '',
  ].join('\n');
}

export function assembleReport({ manifest, coverage, meta }) {
  const blockers = deriveReleaseBlockingCategories(manifest);
  const decision = planReleaseDecision(blockers, coverage.failures);
  const gaps = gapCategories(manifest);
  const rows = buildCategoryRows(manifest, gaps);
  return [
    renderHeadline({ ...meta, verdict: decision.verdict, verdictReason: decision.reason }),
    renderScope(manifest),
    renderCategories(rows),
    renderCoverage(coverage),
    renderDefects(manifest),
    renderRisks(manifest),
    renderRecommendations(decision.verdict),
    renderApprovals(),
  ].join('\n');
}

export function currentCoverageMeta() {
  const generatedAt = new Date();
  const release = process.env.TSR_RELEASE || 'local-workspace';
  return { release, generatedAt };
}

export function loadCargoMetadata() {
  const metadata = JSON.parse(
    execFileSync('cargo', ['metadata', '--format-version', '1', '--locked', '--no-deps'], {
      encoding: 'utf8',
    }),
  );
  const members = new Set(metadata.workspace_members);
  return metadata.packages.filter((pkg) => members.has(pkg.id));
}

const isCli = process.argv[1]?.endsWith('generate-test-summary-report.mjs');
if (isCli) {
  const [manifestPath, coveragePath, thresholdsPath, outputPath] = process.argv.slice(2);
  if (!manifestPath || !coveragePath || !thresholdsPath || !outputPath) {
    process.stderr.write(
      'usage: generate-test-summary-report.mjs <manifest.json> <llvm-cov-json> ' +
        '<thresholds.json> <output.md>\n',
    );
    process.exit(1);
  }
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  const report = JSON.parse(readFileSync(coveragePath, 'utf8'));
  const thresholds = JSON.parse(readFileSync(thresholdsPath, 'utf8'));
  const evaluated = evaluateCoverage(report, loadCargoMetadata(), safeCoverageConfig(thresholds));
  const coverage = {
    rows: evaluated.rows,
    failures: evaluated.failures,
    orphans: evaluated.orphans,
    uncovered: evaluated.uncovered,
    totalCrates: Object.keys(thresholds.crates ?? {}).length,
  };
  const meta = {
    ...currentCoverageMeta(),
    coverageBaselineCommit: thresholds.baseline?.commit ?? 'n/a',
    coverageBaselineDate: thresholds.baseline?.date ?? 'n/a',
  };
  const markdown = assembleReport({ manifest, coverage, meta });
  writeFileSync(outputPath, markdown);
  process.stdout.write(`Test Summary Report generated → ${outputPath}\n`);
}
