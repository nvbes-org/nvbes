export function assertSecrets(
  path,
  text,
  allowedSecrets,
  { errors, githubExpression, cacheEnvironmentSelector, extractSecrets },
) {
  const referencedSecrets = extractSecrets(text);
  const protectedSecretWorkflows = new Map([
    ['STRIPE_CI_SECRET_KEY', new Set(['.github/workflows/stripe-sandbox.yml'])],
    [
      'ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_PEM',
      new Set([
        '.github/workflows/account-acceptance-ingest.yml',
        '.github/workflows/account-release.yml',
      ]),
    ],
    [
      'ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_PEM',
      new Set([
        '.github/workflows/account-acceptance-ingest.yml',
        '.github/workflows/account-release.yml',
      ]),
    ],
    ['NVBES_STAGING_ACCOUNT_PASSWORD', new Set(['.github/workflows/account-release.yml'])],
    ...[
      'IDENTITY_MFA_ENCRYPTION_KEY',
      'IDENTITY_METRICS_TOKEN',
      'IDENTITY_SENTRY_DSN',
      'IDENTITY_SYNTHETIC_PASSWORD',
      'IDENTITY_SYNTHETIC_RECOVERED_PASSWORD',
    ].map((secret) => [secret, new Set(['.github/workflows/deploy-identity.yml'])]),
    ...['IDENTITY_TERRAFORM_STATE_ACCESS_KEY', 'IDENTITY_TERRAFORM_STATE_SECRET_KEY'].map(
      (secret) => [
        secret,
        new Set([
          '.github/workflows/deploy-identity.yml',
          '.github/workflows/validate-identity-restore.yml',
        ]),
      ],
    ),
    ...['ACCOUNT_METRICS_TOKEN', 'IDENTITY_TOKEN_PUBLIC_KEY_PEM', 'ACCOUNT_SENTRY_DSN'].map(
      (secret) => [secret, new Set(['.github/workflows/deploy-account.yml'])],
    ),
    ...['ACCOUNT_TERRAFORM_STATE_ACCESS_KEY', 'ACCOUNT_TERRAFORM_STATE_SECRET_KEY'].map(
      (secret) => [
        secret,
        new Set([
          '.github/workflows/deploy-account.yml',
          '.github/workflows/validate-account-restore.yml',
        ]),
      ],
    ),
    ...[
      'BILLING_METRICS_TOKEN',
      'BILLING_OPERATOR_TOKEN',
      'BILLING_SENTRY_DSN',
      'STRIPE_SECRET_KEY',
      'STRIPE_WEBHOOK_SECRET',
    ].map((secret) => [secret, new Set(['.github/workflows/deploy-billing.yml'])]),
    ...['BILLING_TERRAFORM_STATE_ACCESS_KEY', 'BILLING_TERRAFORM_STATE_SECRET_KEY'].map(
      (secret) => [
        secret,
        new Set([
          '.github/workflows/deploy-billing.yml',
          '.github/workflows/validate-billing-restore.yml',
        ]),
      ],
    ),
  ]);
  for (const secret of referencedSecrets) {
    if (!allowedSecrets.includes(secret)) {
      errors.push(`${path}: secret ${secret} is not allowlisted`);
    }
    const allowedWorkflows = protectedSecretWorkflows.get(secret);
    if (allowedWorkflows && !allowedWorkflows.has(path)) {
      errors.push(`${path}: protected release secret ${secret} is not allowed here`);
    }
  }

  const secretIndex = text.indexOf('secrets.');
  if (secretIndex >= 0) {
    const isValidatedStripeSandboxWorkflow =
      path === '.github/workflows/stripe-sandbox.yml' &&
      /^  workflow_dispatch:\s*$/mu.test(text) &&
      !/^\s+(?:inputs|pull_request|push|schedule):/mu.test(text) &&
      text.includes('name: stripe-ci') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_EVENT_NAME" == "workflow_dispatch" ]]') &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
      text.includes(`ref: ${githubExpression('github.sha')}`) &&
      text.includes('run: node scripts/stripe-billing-smoke.mjs');
    const preceding = text.slice(Math.max(0, secretIndex - 500), secretIndex);
    const isValidatedDastWorkflow =
      path === '.github/workflows/dast.yml' &&
      /^\s+schedule:\s*$/mu.test(text) &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      text.includes('name: account-dast-staging') &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
      text.includes(`ref: ${githubExpression('github.sha')}`) &&
      text.includes('node tools/security/validate-dast-targets.mjs') &&
      text.includes('DAST_ALLOWED_ORIGINS');
    const isValidatedAccountReleaseWorkflow =
      path === '.github/workflows/account-release.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      !text.includes('inputs.') &&
      text.includes('name: production-account') &&
      text.includes('verify-account-release-ref.mjs') &&
      text.includes(`ref: ${githubExpression('github.ref')}`) &&
      text.includes(`NVBES_RELEASE_SHA: ${githubExpression('github.sha')}`) &&
      text.includes('RELEASE_APPROVED: production') &&
      text.includes('scripts/release-gate.sh production');
    const isValidatedAcceptanceIngestWorkflow =
      path === '.github/workflows/account-acceptance-ingest.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      !text.includes('inputs.') &&
      text.includes('name: account-acceptance') &&
      text.includes('verify-account-release-ref.mjs') &&
      text.includes(`ref: ${githubExpression('github.ref')}`) &&
      text.includes(`NVBES_RELEASE_SHA: ${githubExpression('github.sha')}`) &&
      text.includes('run.path !== ".github/workflows/account-acceptance-source.yml"') &&
      text.includes('node tools/account-quality/verify-acceptance-evidence.mjs') &&
      text.includes('prepare-account-acceptance-publication.mjs') &&
      text.includes(`name: account-acceptance-${githubExpression('github.sha')}`);
    const isValidatedEmailDeploymentWorkflow =
      path === '.github/workflows/deploy-email.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      !/^\s+inputs:\s*$/mu.test(text) &&
      text.includes('name: production-email') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
      text.includes(
        `EMAIL_DEPLOY_CONFIRMATION: ${githubExpression('vars.EMAIL_DEPLOY_CONFIRMATION')}`,
      ) &&
      text.includes(
        `EMAIL_DEPLOY_APPROVED_SHA: ${githubExpression('vars.EMAIL_DEPLOY_APPROVED_SHA')}`,
      ) &&
      text.includes('[[ "$EMAIL_DEPLOY_CONFIRMATION" == "deploy-email-production" ]]') &&
      text.includes('[[ "$EMAIL_DEPLOY_APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]') &&
      text.includes('[[ "$EMAIL_DEPLOY_APPROVED_SHA" == "$GITHUB_SHA" ]]') &&
      text.includes(`ref: ${githubExpression('github.sha')}`) &&
      text.includes('production/email/terraform.tfstate') &&
      text.includes('ghcr.io/nvbes-org/nvbes-email-worker') &&
      text.includes('cosign verify') &&
      text.includes('email-runtime.tfplan');
    const isValidatedEmailRestoreWorkflow =
      path === '.github/workflows/validate-email-restore.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      text.includes('name: production-email') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$CONFIRMATION" == "validate-email-production-restore" ]]') &&
      text.includes('[[ "$APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]') &&
      text.includes('[[ "$APPROVED_SHA" == "$(git rev-parse HEAD)" ]]') &&
      text.includes(`ref: ${githubExpression('inputs.approved_sha')}`) &&
      text.includes('production/email/terraform.tfstate') &&
      text.includes('restore_database_id');
    const isValidatedTrustRiskDeploymentWorkflow =
      path === '.github/workflows/deploy-trust-risk.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      !/^\s+inputs:\s*$/mu.test(text) &&
      text.includes('name: production-trust-risk') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
      text.includes(`ref: ${githubExpression('github.sha')}`) &&
      text.includes('production/trust-risk/terraform.tfstate') &&
      text.includes('ghcr.io/nvbes-org/nvbes-trust-risk-service') &&
      text.includes('cosign verify') &&
      text.includes('trust-risk-runtime.tfplan');
    const isValidatedIdentityDeploymentWorkflow =
      path === '.github/workflows/deploy-identity.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      !/^\s+inputs:\s*$/mu.test(text) &&
      text.includes('name: production-identity') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
      text.includes(
        `IDENTITY_DEPLOY_CONFIRMATION: ${githubExpression('vars.IDENTITY_DEPLOY_CONFIRMATION')}`,
      ) &&
      text.includes(
        `IDENTITY_DEPLOY_APPROVED_SHA: ${githubExpression('vars.IDENTITY_DEPLOY_APPROVED_SHA')}`,
      ) &&
      text.includes('[[ "$IDENTITY_DEPLOY_CONFIRMATION" == "deploy-identity-production" ]]') &&
      text.includes('[[ "$IDENTITY_DEPLOY_APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]') &&
      text.includes('[[ "$IDENTITY_DEPLOY_APPROVED_SHA" == "$GITHUB_SHA" ]]') &&
      text.includes(`ref: ${githubExpression('github.sha')}`) &&
      text.includes('production/identity/terraform.tfstate') &&
      text.includes('ghcr.io/nvbes-org/nvbes-identity-service') &&
      text.includes('cosign verify') &&
      text.includes('identity-runtime.tfplan');
    const isValidatedIdentityRestoreWorkflow =
      path === '.github/workflows/validate-identity-restore.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      text.includes('name: production-identity') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$CONFIRMATION" == "validate-identity-production-restore" ]]') &&
      text.includes('[[ "$APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]') &&
      text.includes('[[ "$APPROVED_SHA" == "$(git rev-parse HEAD)" ]]') &&
      text.includes(`ref: ${githubExpression('inputs.approved_sha')}`) &&
      text.includes('production/identity/terraform.tfstate') &&
      text.includes('expected_principal_id') &&
      text.includes('DELETE_RESTORE_DATABASE=true');
    const isValidatedAccountDeploymentWorkflow =
      path === '.github/workflows/deploy-account.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      !/^\s+inputs:\s*$/mu.test(text) &&
      text.includes('name: production-account') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
      text.includes(
        `ACCOUNT_DEPLOY_CONFIRMATION: ${githubExpression('vars.ACCOUNT_DEPLOY_CONFIRMATION')}`,
      ) &&
      text.includes(
        `ACCOUNT_DEPLOY_APPROVED_SHA: ${githubExpression('vars.ACCOUNT_DEPLOY_APPROVED_SHA')}`,
      ) &&
      text.includes('[[ "$ACCOUNT_DEPLOY_CONFIRMATION" == "deploy-account-production" ]]') &&
      text.includes('[[ "$ACCOUNT_DEPLOY_APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]') &&
      text.includes('[[ "$ACCOUNT_DEPLOY_APPROVED_SHA" == "$GITHUB_SHA" ]]') &&
      text.includes(`ref: ${githubExpression('github.sha')}`) &&
      text.includes('production/account/terraform.tfstate') &&
      text.includes('ghcr.io/nvbes-org/nvbes-account-service') &&
      text.includes('cosign verify') &&
      text.includes('account-runtime.tfplan');
    const isValidatedAccountRestoreWorkflow =
      path === '.github/workflows/validate-account-restore.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      text.includes('name: production-account') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$CONFIRMATION" == "validate-account-production-restore" ]]') &&
      text.includes('[[ "$APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]') &&
      text.includes('[[ "$APPROVED_SHA" == "$(git rev-parse HEAD)" ]]') &&
      text.includes(`ref: ${githubExpression('inputs.approved_sha')}`) &&
      text.includes('production/account/terraform.tfstate') &&
      text.includes('expected_principal_id') &&
      text.includes('DELETE_RESTORE_DATABASE=true');
    const isValidatedBillingDeploymentWorkflow =
      path === '.github/workflows/deploy-billing.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      !/^\s+inputs:\s*$/mu.test(text) &&
      text.includes('name: production-billing') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
      text.includes(`ref: ${githubExpression('github.sha')}`) &&
      text.includes('production/billing/terraform.tfstate') &&
      text.includes('ghcr.io/nvbes-org/nvbes-billing-service') &&
      text.includes('cosign verify') &&
      text.includes('billing-runtime.tfplan');
    const isValidatedBillingRestoreWorkflow =
      path === '.github/workflows/validate-billing-restore.yml' &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      text.includes('name: production-billing') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$CONFIRMATION" == "validate-billing-production-restore" ]]') &&
      text.includes('[[ "$APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]') &&
      text.includes('[[ "$APPROVED_SHA" == "$(git rev-parse HEAD)" ]]') &&
      text.includes(`ref: ${githubExpression('inputs.approved_sha')}`) &&
      text.includes('production/billing/terraform.tfstate') &&
      text.includes('restore_database_id');
    const isValidatedCiCacheRotationWorkflow =
      path === '.github/workflows/rotate-ci-cache-credentials.yml' &&
      /^\s+schedule:\s*$/mu.test(text) &&
      /^\s+workflow_dispatch:\s*$/mu.test(text) &&
      text.includes('name: production-bootstrap') &&
      text.includes("if: github.ref == 'refs/heads/main'") &&
      text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
      text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
      text.includes(`ref: ${githubExpression('github.sha')}`) &&
      text.includes('production/bootstrap/terraform.tfstate') &&
      text.includes('repair_bucket_policy:') &&
      text.includes('if: inputs.repair_bucket_policy == true') &&
      text.includes('-refresh=false') &&
      text.includes('-target=module.ci_cache.scaleway_object_bucket_policy.ci_cache') &&
      text.includes('ci-cache-policy-repair.tfplan') &&
      text.includes('-target=module.ci_cache') &&
      text.includes('ci-cache-rotation.tfplan');
    const isValidatedBranchCacheWorkflow =
      path === '.github/workflows/ci.yml' &&
      /^ {2}push:\n {4}branches: \[main\]$/mu.test(text) &&
      text.includes('jobs:\n  authorize-cache:') &&
      text.includes(`ref: ${githubExpression('github.event.pull_request.base.sha')}`) &&
      text.includes('path: trusted-base') &&
      text.includes('node trusted-base/tools/ci/authorize-pr-cache.mjs') &&
      text.includes('scope:\n    needs: authorize-cache') &&
      text.includes(
        `environment:\n      name: ${githubExpression(cacheEnvironmentSelector)}\n      deployment: false`,
      ) &&
      text.includes("if: github.event_name == 'push'") &&
      text.includes('[[ "$GITHUB_EVENT_NAME" == "push" ]]') &&
      text.includes('[[ "$GITHUB_REF" == refs/heads/* ]]') &&
      text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
      text.includes('uses: ./.github/actions/ci-setup') &&
      text.includes(`AWS_ACCESS_KEY_ID: ${githubExpression('secrets.SCW_CI_CACHE_ACCESS_KEY')}`) &&
      text.includes(
        `AWS_SECRET_ACCESS_KEY: ${githubExpression('secrets.SCW_CI_CACHE_SECRET_KEY')}`,
      );
    if (
      !preceding.includes("if: github.event_name == 'push' && github.ref == 'refs/heads/main'") &&
      !isValidatedDastWorkflow &&
      !isValidatedStripeSandboxWorkflow &&
      !isValidatedAccountReleaseWorkflow &&
      !isValidatedAcceptanceIngestWorkflow &&
      !isValidatedEmailDeploymentWorkflow &&
      !isValidatedEmailRestoreWorkflow &&
      !isValidatedTrustRiskDeploymentWorkflow &&
      !isValidatedBillingDeploymentWorkflow &&
      !isValidatedBillingRestoreWorkflow &&
      !isValidatedIdentityDeploymentWorkflow &&
      !isValidatedIdentityRestoreWorkflow &&
      !isValidatedAccountDeploymentWorkflow &&
      !isValidatedAccountRestoreWorkflow &&
      !isValidatedCiCacheRotationWorkflow &&
      !isValidatedBranchCacheWorkflow
    ) {
      errors.push(
        `${path}: secret-bearing step must be restricted to trusted push on main or a validated protected-Environment workflow`,
      );
    }
  }
}
