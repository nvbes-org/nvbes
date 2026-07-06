#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { readPackageScripts, validateProofCommand } from './execution-backlog.proof.mjs';

const args = process.argv.slice(2);
const write = args.includes('--write');
const outputPath = 'docs/migration/billing-multi-psp-continuity.generated.json';
const markdownPath = 'docs/migration/billing-multi-psp-continuity.md';

const sources = {
  provider: 'libs/rust/billing/src/provider.rs',
  providerTests: 'libs/rust/billing/src/provider_tests.rs',
  cbProvider: 'libs/rust/billing/src/cb.rs',
  providerSubscriptions: 'libs/rust/billing/src/db.provider_subscriptions.rs',
  workspaceEffects: 'libs/rust/billing/src/stripe_webhook_workspace_effects.rs',
  portalSubscriptions: 'libs/rust/billing/src/portal_views.subscriptions.rs',
  billingMigration: 'apps/billing-api/migrations/0002_billing_platform_core.sql',
  primaryFallbackMigration:
    'apps/billing-api/migrations/0004_billing_provider_subscription_primary_fallback.sql',
  cbMigration: 'apps/billing-api/migrations/0009_billing_provider_cb.sql',
  billingClientProvider: 'libs/ts/billing-client/src/billing.provider.ts',
  graphqlSchema: 'contracts/graphql/schema.graphql',
};

const errors = [];
const packageScripts = readPackageScripts(errors);

function read(path) {
  if (!existsSync(path)) {
    errors.push(`${path}: missing`);
    return '';
  }
  return readFileSync(path, 'utf8');
}

function textCheck(id, path, description, pattern) {
  const content = read(path);
  return {
    id,
    description,
    path,
    status: content.includes(pattern) ? 'passed' : 'failed',
    pattern,
  };
}

function buildChecks() {
  return [
    textCheck(
      'provider-public-codes',
      sources.provider,
      'Billing provider model exposes Stripe, Mollie and CB',
      'PROVIDER_CODES: &[&str] = &["stripe", "mollie", "cb"]',
    ),
    textCheck(
      'provider-code-test',
      sources.providerTests,
      'Provider code tests cover CB',
      'provider_code("cb")',
    ),
    textCheck(
      'cb-services',
      sources.cbProvider,
      "CB adapter models Safe'R, Updat'R and Fast'R services",
      '["safe_r", "updat_r", "fast_r"]',
    ),
    textCheck(
      'cb-safe-r',
      sources.cbProvider,
      "CB Safe'R excludes recurring payments",
      'safe_r_excludes_recurring',
    ),
    textCheck(
      'cb-updat-r',
      sources.cbProvider,
      "CB Updat'R requires stored credential context",
      'stored_credential_required',
    ),
    textCheck(
      'cb-fast-r',
      sources.cbProvider,
      "CB Fast'R requires customer initiated ecommerce context",
      'customer_initiated_required',
    ),
    textCheck(
      'cb-acquirer-pat-port',
      sources.cbProvider,
      'CB integration remains behind acquirer or PAT port',
      'cb_acquirer_or_pat_required',
    ),
    textCheck(
      'provider-customer-boundary',
      sources.providerTests,
      'Provider customer lookup does not reuse unrelated provider IDs',
      'provider_customer_id_for_does_not_reuse_other_provider_customer',
    ),
    textCheck(
      'provider-subscription-primary-column',
      sources.primaryFallbackMigration,
      'Provider subscriptions store primary provider ownership',
      'primary_for_subscription BOOLEAN NOT NULL DEFAULT FALSE',
    ),
    textCheck(
      'provider-subscription-fallback-column',
      sources.primaryFallbackMigration,
      'Provider subscriptions store fallback eligibility',
      'fallback_eligible BOOLEAN NOT NULL DEFAULT FALSE',
    ),
    textCheck(
      'provider-subscription-one-primary-index',
      sources.primaryFallbackMigration,
      'Database enforces one primary provider subscription',
      'idx_billing_provider_subscriptions_one_primary',
    ),
    textCheck(
      'provider-subscription-demotion',
      sources.providerSubscriptions,
      'New primary provider subscription demotes previous providers',
      'demoted_primary',
    ),
    textCheck(
      'provider-subscription-active-fallback',
      sources.providerSubscriptions,
      'Active non-primary provider subscriptions remain fallback eligible',
      'active_non_primary_provider_subscription_remains_fallback_eligible',
    ),
    textCheck(
      'provider-subscription-primary-not-fallback',
      sources.providerSubscriptions,
      'Primary provider subscriptions are not fallback candidates',
      'primary_provider_subscription_is_never_marked_as_fallback',
    ),
    textCheck(
      'provider-subscription-inactive-not-fallback',
      sources.providerSubscriptions,
      'Inactive provider subscriptions are not fallback candidates',
      'inactive_non_primary_provider_subscription_is_not_fallback_eligible',
    ),
    textCheck(
      'workspace-effects-primary-only',
      sources.workspaceEffects,
      'Webhook workspace effects apply only to primary provider subscription',
      'workspace_effects_apply_only_to_primary_provider_subscription',
    ),
    textCheck(
      'portal-provider-list',
      sources.portalSubscriptions,
      'Portal subscriptions expose provider-neutral primary and fallback state',
      'fallback_eligible',
    ),
    textCheck(
      'cb-migration',
      sources.cbMigration,
      'Billing database enum supports CB provider',
      "ADD VALUE IF NOT EXISTS 'cb'",
    ),
    textCheck(
      'billing-client-cb',
      sources.billingClientProvider,
      'Billing TypeScript client supports CB provider',
      "['stripe', 'mollie', 'cb']",
    ),
    textCheck(
      'graphql-cb',
      sources.graphqlSchema,
      'GraphQL Billing provider enum supports CB',
      'CB',
    ),
  ];
}

function summarize(checks) {
  const failed = checks.filter((check) => check.status === 'failed').length;
  return {
    checks: checks.length,
    passed: checks.length - failed,
    failed,
    status: failed === 0 ? 'passed' : 'failed',
  };
}

function sameItems(actual, expected) {
  return (
    Array.isArray(actual) &&
    actual.length === expected.length &&
    actual.every((item, index) => item === expected[index])
  );
}

function validateReport(report) {
  const seen = new Set();
  for (const check of report.checks) {
    if (seen.has(check.id)) errors.push(`${outputPath}: duplicate check ${check.id}`);
    seen.add(check.id);
    if (!check.description) errors.push(`${check.id}: description is required`);
    if (!Object.values(sources).includes(check.path))
      errors.push(`${check.id}: path is not in billing multi-PSP source contract`);
    if (!['passed', 'failed'].includes(check.status))
      errors.push(`${check.id}: unsupported status ${check.status}`);
    if (!check.pattern) errors.push(`${check.id}: pattern is required`);
  }
  const expectedSummary = summarize(report.checks);
  for (const [field, value] of Object.entries(expectedSummary)) {
    if (report.summary[field] !== value)
      errors.push(`${outputPath}: summary.${field} must be ${value}`);
  }
  if (report.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
  if (report.generation?.command !== 'tools/migration/billing-multi-psp-continuity.mjs --write') {
    errors.push(`${outputPath}: generation.command is invalid`);
  }
  if (!sameItems(report.generation?.sources, Object.values(sources))) {
    errors.push(`${outputPath}: generation.sources must match billing multi-PSP source contract`);
  }
  if (
    !sameItems(report.generation?.targeted_tests, [
      'cargo test -p nvbes-billing provider_subscription --locked',
      'cargo test -p nvbes-billing workspace_effects_apply_only_to_primary_provider_subscription --locked',
      'cargo test -p nvbes-billing provider_code --locked',
      'cargo test -p nvbes-billing cb_ --locked',
    ])
  ) {
    errors.push(`${outputPath}: generation.targeted_tests is invalid`);
  }
  for (const command of report.generation?.targeted_tests ?? []) {
    errors.push(
      ...validateProofCommand(
        { id: 'billing-multi-psp-continuity', proof: command },
        packageScripts,
      ),
    );
  }
}

function serializeJson(data) {
  return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
  const lines = [
    '# Billing Multi-PSP Continuity Evidence',
    '',
    '## Status',
    '',
    `- status: ${data.summary.status}`,
    `- checks: ${data.summary.checks}`,
    `- passed: ${data.summary.passed}`,
    `- failed: ${data.summary.failed}`,
    '',
    '## Rules',
    '',
    '- Billing provider contracts must expose Stripe, Mollie and CB consistently.',
    "- CB Safe'R, Updat'R and Fast'R must stay behind an acquirer/PAT port, not a product runtime dependency.",
    '- Provider subscriptions must support exactly one primary provider and fallback-eligible non-primary providers.',
    '- Webhook side effects must mutate workspace state only for the primary provider subscription.',
    '- Targeted tests must cover provider code parsing and multi-PSP continuity invariants.',
    '',
    '## Evidence',
    '',
    '| Check | Status | Path |',
    '|---|---:|---|',
  ];
  for (const check of data.checks) {
    lines.push(`| ${check.description} | ${check.status} | \`${check.path}\` |`);
  }
  lines.push(
    '',
    '## Decision',
    '',
    data.summary.failed === 0
      ? 'Billing multi-PSP continuity evidence is covered for repository cutover gates. Production cutover still requires provider sandbox E2E evidence.'
      : 'Billing multi-PSP continuity evidence is blocked until failed checks pass.',
    '',
    '## Regeneration',
    '',
    '```bash',
    'pnpm check:migration-billing-multi-psp-continuity',
    'tools/migration/billing-multi-psp-continuity.mjs --write',
    '```',
    '',
  );
  return lines.join('\n');
}

const checks = buildChecks();
const summary = summarize(checks);
const report = {
  schema_version: 1,
  generation: {
    command: 'tools/migration/billing-multi-psp-continuity.mjs --write',
    sources: Object.values(sources),
    targeted_tests: [
      'cargo test -p nvbes-billing provider_subscription --locked',
      'cargo test -p nvbes-billing workspace_effects_apply_only_to_primary_provider_subscription --locked',
      'cargo test -p nvbes-billing provider_code --locked',
      'cargo test -p nvbes-billing cb_ --locked',
    ],
  },
  summary,
  checks,
};

validateReport(report);

const json = serializeJson(report);
const markdown = serializeMarkdown(report);

if (write) {
  mkdirSync(dirname(outputPath), { recursive: true });
  writeFileSync(outputPath, json);
  writeFileSync(markdownPath, markdown);
  console.log(`Billing multi-PSP continuity evidence written to ${markdownPath} and ${outputPath}`);
  process.exit(0);
}

for (const check of checks) {
  if (check.status === 'failed') errors.push(`${check.path}: missing ${check.description}`);
}

for (const [path, expected] of [
  [outputPath, json],
  [markdownPath, markdown],
]) {
  if (!existsSync(path))
    errors.push(`${path}: missing; run tools/migration/billing-multi-psp-continuity.mjs --write`);
  else if (readFileSync(path, 'utf8') !== expected)
    errors.push(`${path}: stale; run tools/migration/billing-multi-psp-continuity.mjs --write`);
}

if (errors.length > 0) {
  console.error('Billing multi-PSP continuity evidence checks failed:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log(
  `Billing multi-PSP continuity evidence: ok (${summary.passed}/${summary.checks} checks passed)`,
);
