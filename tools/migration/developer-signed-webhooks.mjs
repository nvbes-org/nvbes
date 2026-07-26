#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { readPackageScripts, validateProofCommand } from './execution-backlog.proof.mjs';

const args = process.argv.slice(2);
const write = args.includes('--write');
const outputPath = 'docs/migration/developer-signed-webhooks.generated.json';
const markdownPath = 'docs/migration/developer-signed-webhooks.md';

const sources = {
  openapi: 'apps/developer-service/openapi.json',
  openapiExport: 'apps/developer-service/src/developer.http.openapi.rs',
  publicRoutes: 'apps/developer-service/src/developer.http.routes.webhooks.rs',
  consoleRoutes: 'apps/developer-service/src/developer.http.routes.rs',
  delivery: 'apps/developer-service/src/developer.webhooks.signature.rs',
  replayRoutes: 'apps/developer-service/src/developer.grpc.webhooks.rs',
  portalMigration: 'apps/account-service/migrations/0011_developer_portal.sql',
  consoleMigration: 'apps/account-service/migrations/0008_developer_console.sql',
  developerTests: 'apps/developer-service/src/developer.grpc.webhooks.contract_tests.rs',
  developerApi: 'apps/console-web/src/developer.api.ts',
  developerSchemas: 'apps/console-web/src/developer.schemas.ts',
  consoleWebTests: 'apps/console-web/src/__tests__/developer.webhooks.test.ts',
  webhookHelpers: 'apps/console-web/src/pages/WebhooksPage.helpers.ts',
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

function parseJson(path, content) {
  try {
    return JSON.parse(content);
  } catch (error) {
    errors.push(`${path}: invalid JSON: ${error.message}`);
    return undefined;
  }
}

function openApiMethodCheck(id, pathKey, method, description) {
  const content = read(sources.openapi);
  if (!content) {
    return {
      id,
      description,
      path: sources.openapi,
      status: 'failed',
      pattern: `${pathKey}.${method}`,
    };
  }
  const api = parseJson(sources.openapi, content);
  return {
    id,
    description,
    path: sources.openapi,
    status: api.paths?.[pathKey]?.[method] ? 'passed' : 'failed',
    pattern: `paths['${pathKey}'].${method}`,
  };
}

function buildChecks() {
  return [
    openApiMethodCheck(
      'openapi-webhooks-list',
      '/developer/webhooks',
      'get',
      'OpenAPI exposes developer webhook listing',
    ),
    openApiMethodCheck(
      'openapi-webhooks-create',
      '/developer/webhooks',
      'post',
      'OpenAPI exposes developer webhook creation',
    ),
    openApiMethodCheck(
      'openapi-webhooks-delete',
      '/developer/webhooks/{endpointId}',
      'delete',
      'OpenAPI exposes developer webhook revocation',
    ),
    textCheck(
      'openapi-export-webhooks',
      sources.openapiExport,
      'OpenAPI export includes developer webhook handlers',
      'crate::http::routes::webhooks::create_portal_webhook',
    ),
    textCheck(
      'public-route-secret',
      sources.publicRoutes,
      'Webhook creation returns signing secret once',
      'CreateDeveloperWebhookEndpointResponse',
    ),
    textCheck(
      'public-route-manage-permission',
      sources.publicRoutes,
      'Webhook mutations require manage permission',
      'DeveloperPermission::WebhooksManage',
    ),
    textCheck(
      'console-replay-route',
      sources.consoleRoutes,
      'Developer console exposes delivery replay route',
      '/developer/console/webhooks/deliveries/{deliveryId}/replay',
    ),
    textCheck(
      'delivery-hmac',
      sources.delivery,
      'Webhook signatures use HMAC-SHA256',
      'type HmacSha256 = Hmac<Sha256>',
    ),
    textCheck(
      'delivery-header',
      sources.delivery,
      'Signature header includes timestamp and v1 digest',
      'format!("t={timestamp},{WEBHOOK_SIGNATURE_VERSION}={signature}")',
    ),
    textCheck(
      'delivery-event-binding',
      sources.delivery,
      'Signature binds event id into signed payload',
      'mac.update(event_id.to_string().as_bytes())',
    ),
    textCheck(
      'delivery-payload-binding',
      sources.delivery,
      'Signature binds raw payload bytes',
      'mac.update(payload)',
    ),
    textCheck(
      'delivery-window',
      sources.delivery,
      'Signature freshness window rejects replay outside tolerance',
      'WEBHOOK_SIGNATURE_TOLERANCE_SECONDS',
    ),
    textCheck(
      'delivery-replay-classifier',
      sources.delivery,
      'Replay classifier only allows failed or pending deliveries',
      '"failed" | "pending" => WebhookReplayDecision::Replayable',
    ),
    textCheck(
      'delivery-signature-test',
      sources.delivery,
      'Unit test proves signature binding',
      'developer_webhook_signature_binds_event_timestamp_and_payload',
    ),
    textCheck(
      'delivery-window-test',
      sources.delivery,
      'Unit test proves replay window rejection',
      'developer_webhook_signature_timestamp_rejects_replay_window',
    ),
    textCheck(
      'delivery-replay-test',
      sources.delivery,
      'Unit test proves replay idempotency guard',
      'developer_webhook_replay_decision_is_idempotency_guard',
    ),
    textCheck(
      'replay-status-guard',
      sources.replayRoutes,
      'Replay service applies replayability guard',
      'ensure_replayable(original.get("status"))',
    ),
    textCheck(
      'replay-conflict',
      sources.replayRoutes,
      'Replay insert is idempotent for the original delivery',
      'ON CONFLICT (tenant_id, replayed_from_delivery_id)',
    ),
    textCheck(
      'portal-replay-index',
      sources.portalMigration,
      'Portal schema prevents duplicate replays per original delivery',
      'idx_developer_webhook_deliveries_replay_once',
    ),
    textCheck(
      'console-replay-index',
      sources.consoleMigration,
      'Console schema prevents duplicate replays per original delivery',
      'idx_developer_webhook_deliveries_replay_once',
    ),
    textCheck(
      'backend-replay-route-test',
      sources.developerTests,
      'Backend contract test covers replay idempotency and response shape',
      'api_log_reads_and_replay_use_webhook_deliveries',
    ),
    textCheck(
      'console-web-list',
      sources.developerApi,
      'Console web lists console webhook endpoints',
      'listDeveloperConsoleWebhooks',
    ),
    textCheck(
      'console-web-deliveries',
      sources.developerApi,
      'Console web lists delivery attempts',
      'listDeveloperConsoleWebhookDeliveries',
    ),
    textCheck(
      'console-web-replay',
      sources.developerApi,
      'Console web calls replay endpoint',
      'replayDeveloperConsoleWebhookDelivery',
    ),
    textCheck(
      'console-web-schema',
      sources.developerSchemas,
      'Console web validates webhook delivery status',
      'DeveloperWebhookDeliverySchema',
    ),
    textCheck(
      'console-web-helper',
      sources.webhookHelpers,
      'Console web gates replay actions by delivery status',
      'canReplayWebhookDelivery',
    ),
    textCheck(
      'console-web-helper-test',
      sources.consoleWebTests,
      'Console web test blocks delivered delivery replay',
      'blocks delivered deliveries',
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
      errors.push(`${check.id}: path is not in developer signed webhook source contract`);
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
  if (report.generation?.command !== 'node tools/migration/developer-signed-webhooks.mjs --write') {
    errors.push(`${outputPath}: generation.command is invalid`);
  }
  if (!sameItems(report.generation?.sources, Object.values(sources))) {
    errors.push(
      `${outputPath}: generation.sources must match developer signed webhook source contract`,
    );
  }
  if (
    !sameItems(report.generation?.targeted_tests, [
      'cargo test -p nvbes-developer-service developer_webhook --locked',
      'pnpm --dir apps/console-web test -- --run developer.webhooks.test.ts',
    ])
  ) {
    errors.push(`${outputPath}: generation.targeted_tests is invalid`);
  }
  for (const command of report.generation?.targeted_tests ?? []) {
    errors.push(
      ...validateProofCommand({ id: 'developer-signed-webhooks', proof: command }, packageScripts),
    );
  }
}

function serializeJson(data) {
  return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
  const lines = [
    '# Developer Signed Webhook Evidence',
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
    '- Every evidence row must be generated from the developer signed webhook source contract.',
    '- `passed` requires the configured file or OpenAPI document to contain the expected pattern.',
    '- Summary counters must match evidence rows.',
    '- Targeted tests must name the backend and frontend webhook checks required by parity.',
    '- Generation provenance must identify sources, write command and targeted tests.',
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
      ? 'Developer signed webhook and replay-idempotency evidence is covered for repository cutover gates.'
      : 'Developer signed webhook and replay-idempotency evidence is blocked until failed checks pass.',
    '',
    '## Regeneration',
    '',
    '```bash',
    'pnpm check:migration-developer-signed-webhooks',
    'node tools/migration/developer-signed-webhooks.mjs --write',
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
    command: 'node tools/migration/developer-signed-webhooks.mjs --write',
    sources: Object.values(sources),
    targeted_tests: [
      'cargo test -p nvbes-developer-service developer_webhook --locked',
      'pnpm --dir apps/console-web test -- --run developer.webhooks.test.ts',
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
  console.log(`Developer signed webhook evidence written to ${markdownPath} and ${outputPath}`);
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
    errors.push(`${path}: missing; run node tools/migration/developer-signed-webhooks.mjs --write`);
  else if (readFileSync(path, 'utf8') !== expected)
    errors.push(`${path}: stale; run node tools/migration/developer-signed-webhooks.mjs --write`);
}

if (errors.length > 0) {
  console.error('Developer signed webhook evidence checks failed:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log(
  `Developer signed webhook evidence: ok (${summary.passed}/${summary.checks} checks passed)`,
);
