const rustWorkspacePaths = new Set([
  'Cargo.lock',
  'Cargo.toml',
  'rust-toolchain.toml',
  'rustfmt.toml',
]);

export function isTrustedPush(eventName, ref) {
  return eventName === 'push' && ref === 'refs/heads/main';
}

export function isUsableCommitSha(value) {
  return /^[0-9a-f]{40}$/u.test(value) && !/^0{40}$/u.test(value);
}

function matchesPath(path, prefixes) {
  return prefixes.some(
    (prefix) => path === prefix || path.startsWith(`${prefix}/`) || path.startsWith(`${prefix}.`),
  );
}

export function isRustWorkspaceAffected(changedPaths) {
  return changedPaths.some(
    (path) =>
      rustWorkspacePaths.has(path) ||
      path === 'scripts/test-unit.sh' ||
      path === 'docs/testing/v1/manifest.json' ||
      path.startsWith('tools/rust-workspace/') ||
      path.startsWith('libs/ts/email-ui/') ||
      path.startsWith('libs/ts/identity-sdk-core/') ||
      path.startsWith('.cargo/') ||
      path.startsWith('vendor/xmlsec/') ||
      ((path.startsWith('apps/') || path.startsWith('libs/rust/')) &&
        (path.endsWith('.rs') || path.endsWith('Cargo.toml') || path.endsWith('Cargo.lock'))) ||
      path.startsWith('contracts/protobuf/') ||
      path.startsWith('libs/rust/email/templates/'),
  );
}

export function isDatabaseScopeAffected(changedPaths, scope) {
  const patterns = {
    api: ['apps/api/migrations', 'apps/api/src/api.migrations', 'apps/api/src/api.ports'],
    worker: ['apps/worker/src/worker.outbox', 'apps/worker/src/worker.privacy'],
    account: [
      'libs/rust/account/migrations',
      'libs/rust/account/src/account.db',
      'apps/account-service/migrations',
      'apps/account-service/src/account.db',
      'apps/account-service/src/account.database',
      'scripts/test-account-service-database.sh',
      'scripts/validate-account-test-database',
    ],
    billing: [
      'libs/rust/billing-service/migrations',
      'libs/rust/billing-service/src/billing.db',
      'apps/billing-service/migrations',
      'apps/billing-service/src/billing.db',
      'apps/billing-service/src/billing.database',
      'scripts/test-billing-service-database.sh',
      'scripts/validate-billing-test-database',
    ],
    email: [
      'libs/rust/email/migrations',
      'apps/email-worker/migrations',
      'apps/email-worker/src/email.worker.database',
      'apps/email-worker/src/email.worker.dispatch.db',
      'apps/email-worker/src/email.worker.metrics.db',
      'apps/email-worker/src/email.worker.webhook.db',
      'scripts/test-email-worker-database.sh',
      'scripts/validate-email-test-database',
    ],
    identity: [
      'libs/rust/identity/migrations',
      'libs/rust/identity/src/identity.db',
      'apps/identity-service/migrations',
      'apps/identity-service/src/identity.db',
      'apps/identity-service/src/identity.database',
      'scripts/test-identity-service-database.sh',
      'scripts/validate-identity-test-database',
    ],
    'trust-risk': [
      'libs/rust/trust-risk/migrations',
      'apps/trust-risk-service/migrations',
      'apps/trust-risk-service/src/trust_risk.db',
      'apps/trust-risk-service/src/trust_risk.database',
      'scripts/test-trust-risk-service-database.sh',
      'scripts/validate-trust-risk-test-database',
    ],
  };
  return changedPaths.some((path) => matchesPath(path, patterns[scope] ?? []));
}
