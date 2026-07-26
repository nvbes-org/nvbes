#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import { readPackageScripts, validateProofCommand } from './execution-backlog.proof.mjs';

const args = process.argv.slice(2);
const write = args.includes('--write');
const outputPath = 'docs/migration/developer-oauth-tokens.generated.json';
const markdownPath = 'docs/migration/developer-oauth-tokens.md';

const sources = {
  identityOpenapi: 'apps/account-service/openapi.json',
  developerOpenapi: 'apps/developer-service/openapi.json',
  identityOpenapiExport: 'apps/account-service/src/identity.http.openapi.rs',
  developerOpenapiExport: 'apps/developer-service/src/developer.http.openapi.rs',
  oauthRoutes: 'apps/account-service/src/identity.domains.oauth.routes.clients.rs',
  oauthCreate: 'apps/account-service/src/identity.domains.oauth.clients.create.rs',
  oauthRevoke: 'apps/account-service/src/identity.domains.oauth.clients.revoke.rs',
  clientCredentialsTests:
    'apps/account-service/src/identity.domains.oauth.flows.client_credentials.tests.rs',
  tokenRoutes: 'apps/developer-service/src/developer.http.routes.tools.rs',
  developerOAuthRoutes: 'apps/developer-service/src/developer.http.routes.oauth.rs',
  developerTypes: 'apps/developer-service/src/developer.http.types.rs',
  sdkCore: 'libs/ts/identity-sdk-core/src/types.gen.ts',
  identityClient: 'libs/ts/identity-client/src/index.ts',
  developerApi: 'apps/console-web/src/developer.api.ts',
  developerSchemas: 'apps/console-web/src/developer.schemas.ts',
  developerSchemaTests: 'apps/console-web/src/__tests__/developer.schemas.test.ts',
  developerTokenTests: 'apps/console-web/src/__tests__/developer.token-health.test.ts',
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

function openApiMethodCheck(source, id, pathKey, method, description) {
  const content = read(source);
  if (!content) {
    return {
      id,
      description,
      path: source,
      status: 'failed',
      pattern: `${pathKey}.${method}`,
    };
  }
  const api = parseJson(source, content);
  return {
    id,
    description,
    path: source,
    status: api.paths?.[pathKey]?.[method] ? 'passed' : 'failed',
    pattern: `paths['${pathKey}'].${method}`,
  };
}

function buildChecks() {
  return [
    openApiMethodCheck(
      sources.identityOpenapi,
      'openapi-oauth-clients-list',
      '/oauth/clients',
      'get',
      'Identity OpenAPI exposes OAuth client listing',
    ),
    openApiMethodCheck(
      sources.identityOpenapi,
      'openapi-oauth-clients-create',
      '/oauth/clients',
      'post',
      'Identity OpenAPI exposes OAuth client creation',
    ),
    openApiMethodCheck(
      sources.identityOpenapi,
      'openapi-oauth-clients-revoke',
      '/oauth/clients/{clientId}',
      'delete',
      'Identity OpenAPI exposes OAuth client revocation',
    ),
    openApiMethodCheck(
      sources.identityOpenapi,
      'openapi-oauth-policies',
      '/oauth/clients/{clientId}/policies',
      'post',
      'Identity OpenAPI exposes OAuth client policy creation',
    ),
    openApiMethodCheck(
      sources.developerOpenapi,
      'openapi-token-inspect',
      '/developer/tokens/inspect',
      'post',
      'Developer OpenAPI exposes token inspection',
    ),
    textCheck(
      'openapi-export-oauth',
      sources.identityOpenapiExport,
      'Identity OpenAPI export includes OAuth client handlers',
      'crate::domains::oauth::routes::clients::create_client',
    ),
    textCheck(
      'openapi-export-token-inspect',
      sources.developerOpenapiExport,
      'Developer OpenAPI export includes token inspection handler',
      'crate::http::routes::tools::inspect_token',
    ),
    textCheck(
      'oauth-route-create',
      sources.oauthRoutes,
      'OAuth routes wire list, create, and revoke',
      'get(list_clients)\n                .post(create_client)',
    ),
    textCheck(
      'oauth-route-step-up',
      sources.oauthRoutes,
      'OAuth management mutations require recent step-up',
      'require_management_step_up',
    ),
    textCheck(
      'oauth-create-secret',
      sources.oauthCreate,
      'OAuth creation returns generated client secret once',
      'client_secret',
    ),
    textCheck(
      'oauth-revoke-refresh',
      sources.oauthRevoke,
      'OAuth revocation invalidates client refresh tokens',
      'revoke_all_client_refresh_tokens',
    ),
    textCheck(
      'oauth-revoke-par',
      sources.oauthRevoke,
      'OAuth revocation invalidates pushed authorization requests',
      'revoke_pushed_authorization_requests_for_client',
    ),
    textCheck(
      'oauth-revoke-codes',
      sources.oauthRevoke,
      'OAuth revocation invalidates authorization codes',
      'revoke_authorization_codes_for_client',
    ),
    textCheck(
      'oauth-revoke-device-codes',
      sources.oauthRevoke,
      'OAuth revocation invalidates device codes',
      'revoke_device_codes_for_client',
    ),
    textCheck(
      'machine-token-audit-test',
      sources.clientCredentialsTests,
      'Machine token test records last-used and audit metadata',
      'client_credentials_records_last_used_and_machine_token_audit',
    ),
    textCheck(
      'developer-token-inspect-permission',
      sources.tokenRoutes,
      'Developer token inspection requires permission',
      'DeveloperPermission::TokensInspect',
    ),
    textCheck(
      'developer-token-debug-route',
      sources.tokenRoutes,
      'Developer console token debug records hash prefix only',
      'token_hash_prefix',
    ),
    textCheck(
      'developer-token-debug-test',
      sources.tokenRoutes,
      'Backend token debug unit test covers tenant mismatch precedence',
      'token_decision_prioritizes_tenant_mismatch',
    ),
    textCheck(
      'developer-oauth-list-route',
      sources.developerOAuthRoutes,
      'Developer console lists OAuth clients with consent and health status',
      'DeveloperOAuthClientSummary',
    ),
    textCheck(
      'sdk-oauth-path',
      sources.sdkCore,
      'Generated SDK core includes OAuth clients path',
      '"/oauth/clients"',
    ),
    textCheck(
      'sdk-oauth-types',
      sources.sdkCore,
      'Generated SDK core includes OAuth client result types',
      'CreateOAuthClientResult',
    ),
    textCheck(
      'developer-token-inspect-types',
      sources.developerTypes,
      'Developer service owns token inspection result types',
      'InspectDeveloperTokenResponse',
    ),
    textCheck(
      'identity-client-oauth-list',
      sources.identityClient,
      'Handwritten identity client lists OAuth clients',
      'listOAuthClients',
    ),
    textCheck(
      'identity-client-oauth-revoke',
      sources.identityClient,
      'Handwritten identity client revokes OAuth clients',
      'revokeOAuthClient',
    ),
    textCheck(
      'console-web-oauth-list',
      sources.developerApi,
      'Console web consumes OAuth client list API',
      'listDeveloperOAuthClients',
    ),
    textCheck(
      'console-web-token-debug',
      sources.developerApi,
      'Console web consumes token debug API',
      'debugDeveloperToken',
    ),
    textCheck(
      'console-web-oauth-schema',
      sources.developerSchemas,
      'Console web validates OAuth client summaries',
      'DeveloperOAuthClientsSchema',
    ),
    textCheck(
      'console-web-token-schema',
      sources.developerSchemas,
      'Console web validates debug token responses',
      'DebugDeveloperTokenSchema',
    ),
    textCheck(
      'console-web-oauth-schema-test',
      sources.developerSchemaTests,
      'Console web schema test parses OAuth client summaries',
      'parses OAuth client summaries',
    ),
    textCheck(
      'console-web-token-schema-test',
      sources.developerTokenTests,
      'Console web token test rejects raw token material in parsed debug response',
      'parses token debug responses without raw token material',
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
      errors.push(`${check.id}: path is not in developer OAuth/token source contract`);
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
  if (report.generation?.command !== 'node tools/migration/developer-oauth-tokens.mjs --write') {
    errors.push(`${outputPath}: generation.command is invalid`);
  }
  if (!sameItems(report.generation?.sources, Object.values(sources))) {
    errors.push(
      `${outputPath}: generation.sources must match developer OAuth/token source contract`,
    );
  }
  if (
    !sameItems(report.generation?.targeted_tests, [
      'cargo test -p nvbes-developer-service token_decision_prioritizes_tenant_mismatch --locked',
      'cargo test -p nvbes-account-service machine_token_audit_metadata_records_grant_client_scope_audience_and_jti --locked',
      'pnpm --dir apps/console-web test -- --run developer.schemas.test.ts developer.token-health.test.ts',
    ])
  ) {
    errors.push(`${outputPath}: generation.targeted_tests is invalid`);
  }
  for (const command of report.generation?.targeted_tests ?? []) {
    errors.push(
      ...validateProofCommand({ id: 'developer-oauth-tokens', proof: command }, packageScripts),
    );
  }
}

function serializeJson(data) {
  return `${JSON.stringify(data, null, 2)}\n`;
}

function serializeMarkdown(data) {
  const lines = [
    '# Developer OAuth and Token Evidence',
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
    '- Every evidence row must be generated from the developer OAuth/token source contract.',
    '- `passed` requires the configured file or OpenAPI document to contain the expected pattern.',
    '- Summary counters must match evidence rows.',
    '- Targeted tests must name the backend and frontend token checks required by parity.',
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
      ? 'Developer OAuth app and token inspection evidence is covered for repository cutover gates.'
      : 'Developer OAuth app and token inspection evidence is blocked until failed checks pass.',
    '',
    '## Regeneration',
    '',
    '```bash',
    'pnpm check:migration-developer-oauth-tokens',
    'node tools/migration/developer-oauth-tokens.mjs --write',
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
    command: 'node tools/migration/developer-oauth-tokens.mjs --write',
    sources: Object.values(sources),
    targeted_tests: [
      'cargo test -p nvbes-developer-service token_decision_prioritizes_tenant_mismatch --locked',
      'cargo test -p nvbes-account-service machine_token_audit_metadata_records_grant_client_scope_audience_and_jti --locked',
      'pnpm --dir apps/console-web test -- --run developer.schemas.test.ts developer.token-health.test.ts',
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
  console.log(`Developer OAuth/token evidence written to ${markdownPath} and ${outputPath}`);
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
    errors.push(`${path}: missing; run node tools/migration/developer-oauth-tokens.mjs --write`);
  else if (readFileSync(path, 'utf8') !== expected)
    errors.push(`${path}: stale; run node tools/migration/developer-oauth-tokens.mjs --write`);
}

if (errors.length > 0) {
  console.error('Developer OAuth/token evidence checks failed:');
  for (const error of errors) console.error(`- ${error}`);
  process.exit(1);
}

console.log(
  `Developer OAuth/token evidence: ok (${summary.passed}/${summary.checks} checks passed)`,
);
