import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const serviceRoot = join(workspaceRoot, 'apps/identity-service');
const dockerfile = readFileSync(join(serviceRoot, 'Dockerfile'), 'utf8');
const mainSource = readFileSync(join(serviceRoot, 'src/main.rs'), 'utf8');
const errorReportingSource = readFileSync(
  join(serviceRoot, 'src/identity.error_reporting.rs'),
  'utf8',
);
const healthSource = readFileSync(join(serviceRoot, 'src/identity.health.rs'), 'utf8');
const deploymentWorkflow = readFileSync(
  join(workspaceRoot, '.github/workflows/deploy.yml'),
  'utf8',
);
const runtimeTerraform = readFileSync(
  join(workspaceRoot, 'infrastructure/environments/identity-production/runtime.tf'),
  'utf8',
);
const productionVariables = readFileSync(
  join(workspaceRoot, 'infrastructure/environments/identity-production/variables.tf'),
  'utf8',
);
const syntheticProof = readFileSync(
  join(workspaceRoot, 'tools/deployment/prove-identity-synthetic-auth.sh'),
  'utf8',
);
const publicRuntimeProof = readFileSync(
  join(workspaceRoot, 'tools/deployment/prove-identity-public-runtime.sh'),
  'utf8',
);

test('image is reproducible and runs Identity as non-root', () => {
  assert.match(dockerfile, /^FROM rust:1\.98\.1-slim-bookworm@sha256:[a-f0-9]{64} AS builder$/m);
  assert.match(dockerfile, /cargo build --locked --release --bin nvbes-identity-service/);
  assert.equal(dockerfile.match(/^ARG DEBIAN_FRONTEND=noninteractive$/gm)?.length, 2);
  assert.ok(dockerfile.includes('CARGO_BUILD_JOBS=1'));
  assert.ok(dockerfile.includes('USER 10001:10001'));
  assert.ok(dockerfile.includes('EXPOSE 8080'));
  assert.ok(dockerfile.includes('ENTRYPOINT ["/app/identity-service"]'));
});

test('container has shallow liveness and graceful shutdown', () => {
  assert.match(
    dockerfile,
    /HEALTHCHECK[^\n]*\\\n\s+CMD \["curl", "--fail", "--silent", "--show-error", "http:\/\/127\.0\.0\.1:8080\/health\/live"\]/,
  );
  assert.equal(dockerfile.includes('/health/ready'), false);
  assert.ok(dockerfile.includes('STOPSIGNAL SIGTERM'));
  assert.ok(mainSource.includes('action == "migrate"'));
  assert.ok(mainSource.includes('SignalKind::terminate()'));
  assert.ok(healthSource.includes('sqlx::query_scalar::<_, i32>("SELECT 1")'));
});

test('runtime exposes gated signup and oauth login surface', () => {
  assert.ok(mainSource.includes('http::router(&state)'));
  assert.ok(mainSource.includes('oauth::router(&state)'));
  assert.ok(errorReportingSource.includes('init_error_reporting_with_config'));
  assert.ok(mainSource.includes('otlp_authorization_header'));
  assert.ok(mainSource.includes('action == "error-reporting-smoke"'));
  assert.ok(errorReportingSource.includes('capture_error_reporting_smoke'));
});

test('container wires OAuth 2.1, discovery, and authorization routes', () => {
  assert.ok(mainSource.includes('oauth::router(&state)'));
  assert.ok(mainSource.includes('discovery::router(&state)'));
  assert.ok(mainSource.includes('authz::router(&state)'));
  assert.ok(mainSource.includes('http::router(&state)'));
});

test('deployment is main-only, approved, immutable and scale-to-zero', () => {
  assert.ok(deploymentWorkflow.includes("github.ref == 'refs/heads/main'"));
  assert.ok(deploymentWorkflow.includes('[[ "$IDENTITY_DEPLOY_APPROVED_SHA" == "$GITHUB_SHA" ]]'));
  assert.ok(deploymentWorkflow.includes('cosign verify'));
  assert.ok(deploymentWorkflow.includes('IDENTITY_IMAGE_DIGEST'));
  assert.ok(runtimeTerraform.includes('min_scale              = 0'));
  assert.ok(runtimeTerraform.includes('max_scale              = 1'));
  assert.ok(runtimeTerraform.includes('privacy                = "public"'));
});

test('production accepts canonical 32-byte MFA keys without UTF-8 assumptions', () => {
  assert.ok(
    productionVariables.includes(
      'regex("^[A-Za-z0-9+/]{42}[AEIMQUYcgkosw048]=$", var.identity_mfa_encryption_key)',
    ),
  );
  assert.equal(productionVariables.includes('length(base64decode('), false);
  const nonUtf8Key = Buffer.alloc(32, 0xff).toString('base64');
  assert.match(nonUtf8Key, /^[A-Za-z0-9+/]{42}[AEIMQUYcgkosw048]=$/);
  assert.equal(Buffer.from(nonUtf8Key, 'base64').byteLength, 32);
});

test('deployment migrates before apply and proves public signup stays gated', () => {
  const identitySection = deploymentWorkflow.slice(deploymentWorkflow.indexOf('deploy-identity:'));
  const migration = identitySection.indexOf('- name: Run database migrations');
  const apply = identitySection.indexOf('- name: Apply reviewed Identity runtime plan');
  assert.ok(migration >= 0);
  assert.ok(apply > migration);
  assert.ok(identitySection.includes('tools/deployment/prove-identity-public-runtime.sh'));
  assert.ok(publicRuntimeProof.includes('/api/v1/auth/register"'));
  assert.ok(publicRuntimeProof.includes('[[ "$registration_status" == "403" ]]'));
  assert.ok(publicRuntimeProof.includes('/api/v1/auth/login"'));
  assert.ok(publicRuntimeProof.includes('/oauth/authorize?'));
});

test('deployment records bounded activation and access-control evidence', () => {
  assert.ok(publicRuntimeProof.includes('--max-time 60'));
  assert.ok(publicRuntimeProof.includes('activation_seconds="$SECONDS"'));
  assert.ok(publicRuntimeProof.includes('[[ "$anonymous_metrics_status" == "401" ]]'));
  assert.ok(publicRuntimeProof.includes('Identity public runtime'));
});

test('deployment proves a private synthetic account lifecycle', () => {
  assert.equal(
    mainSource.match(/database::migrate\(&pool\)\.await\?/g)?.length,
    1,
    'only the dedicated migrate command may change the production schema',
  );
  assert.ok(runtimeTerraform.includes('identity_synthetic_auth'));
  assert.ok(runtimeTerraform.includes('args                   = ["synthetic-auth-smoke"]'));
  assert.ok(deploymentWorkflow.includes('tools/deployment/prove-identity-synthetic-auth.sh'));
  assert.ok(
    syntheticProof.includes(
      'identity-${GITHUB_RUN_ID:?}-${GITHUB_RUN_ATTEMPT:?}@synthetic.invalid',
    ),
  );
  assert.ok(syntheticProof.includes('.audit_events == 5'));
  assert.ok(syntheticProof.includes('.sessions == 2'));
  assert.ok(syntheticProof.includes('.recovery_consumed == true'));
  assert.ok(syntheticProof.includes('.old_sessions_revoked == true'));
  assert.ok(syntheticProof.includes("--file=- <<'SQL'"));
  assert.equal(syntheticProof.includes('--command "SELECT'), false);
});

test('deployment proves Sentry delivery from a private production job', () => {
  assert.ok(runtimeTerraform.includes('identity_error_reporting_smoke'));
  assert.ok(runtimeTerraform.includes('args                   = ["error-reporting-smoke"]'));
  assert.ok(deploymentWorkflow.includes('Prove Identity Sentry delivery'));
  assert.ok(deploymentWorkflow.includes('identity_error_reporting_smoke_job_id'));
  assert.ok(mainSource.includes('result.configured && result.flushed'));
});
