import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { compilationCacheEnvironment } from './configure-sccache.core.mjs';

const deployWorkflow = readFileSync('.github/workflows/deploy.yml', 'utf8');
const rotationWorkflow = readFileSync('.github/workflows/rotate-ci-cache-credentials.yml', 'utf8');
const ciWorkflow = readFileSync('.github/workflows/ci.yml', 'utf8');
const cacheModule = readFileSync('infrastructure/modules/scaleway-ci-cache/main.tf', 'utf8');
const bootstrapReadme = readFileSync('infrastructure/bootstrap/production/README.md', 'utf8');
const securityChecker =
  readFileSync('tools/security/ci-secret-policy.mjs', 'utf8') +
  readFileSync('tools/security/check-ci-cd-security.mjs', 'utf8');

test('deployment builds combine shared Rust and isolated Scaleway registry cache scopes', () => {
  for (const scope of ['email-worker', 'trust-risk-service', 'billing-service']) {
    assert.match(deployWorkflow, /name: production-ci-cache/u);
    assert.match(deployWorkflow, /password: \$\{\{ secrets\.SCW_CI_CACHE_SECRET_KEY \}\}/u);
    assert.ok(
      deployWorkflow.includes(
        `type=registry,ref=\${{ env.CI_CACHE_REGISTRY }}/${scope}:buildcache`,
      ),
    );
    assert.ok(
      deployWorkflow.includes(
        `type=registry,ref=\${{ env.CI_CACHE_REGISTRY }}/${scope}:buildcache,mode=max,compression=zstd,oci-mediatypes=true,image-manifest=true`,
      ),
    );
  }
  assert.ok(
    (deployWorkflow.match(/rust-builder:rust-1\.98\.1-linux-amd64/gu)?.length ?? 0) >= 6,
    'each deploy build imports and exports the shared Linux Rust cache',
  );
});

test('Rust Dockerfiles use stable, platform-scoped BuildKit cache mounts', () => {
  for (const service of [
    'account-service',
    'billing-service',
    'email-worker',
    'identity-service',
    'platform-operations-service',
    'trust-risk-service',
  ]) {
    const dockerfile = readFileSync(`apps/${service}/Dockerfile`, 'utf8');
    assert.match(dockerfile, /id=nvbes-cargo-registry-rust-1\.98\.1-\$\{TARGETPLATFORM\}/u);
    assert.match(dockerfile, /id=nvbes-cargo-git-rust-1\.98\.1-\$\{TARGETPLATFORM\}/u);
    assert.match(dockerfile, /sharing=locked/u);
  }
});

test('cache credentials rotate through two staggered slots', () => {
  assert.match(cacheModule, /credential_slots = \{/u);
  assert.match(cacheModule, /b = floor\(var\.credential_rotation_days \/ 2\)/u);
  assert.match(cacheModule, /active_credential_slot = timecmp\(/u);
  assert.match(cacheModule, /for_each = local\.credential_slots/u);
  assert.match(cacheModule, /create_before_destroy = true/u);
  assert.match(cacheModule, /scaleway_account_project\.ci_cache\.id/u);
  assert.match(cacheModule, /branch_pattern = "main"/u);
  assert.match(cacheModule, /branch_pattern = "dev"/u);
  assert.doesNotMatch(cacheModule, /branch_pattern = "staging"/u);
  assert.match(cacheModule, /branch_pattern = "release\/\*"/u);
  assert.match(cacheModule, /resource "time_rotating" "branch_cache"/u);
  assert.match(cacheModule, /resource "scaleway_iam_api_key" "branch_cache"/u);
});

test('cache access is restricted to the CI application and dedicated bucket', () => {
  assert.match(cacheModule, /resource "scaleway_object_bucket_policy" "ci_cache"/u);
  assert.match(
    cacheModule,
    /Principal = \{ SCW = "application_id:\$\{scaleway_iam_application\.ci_cache\.id\}" \}/u,
  );
  assert.match(cacheModule, /Action {4}= \["s3:ListBucket"\]/u);
  assert.match(cacheModule, /Action {4}= \["s3:GetObject", "s3:PutObject"\]/u);
  assert.match(cacheModule, /"aws:SecureTransport" = "true"/u);
  const bucketConfigurationStart = cacheModule.indexOf(
    'Sid       = "ReadBucketConfigurationForAuthorizedPrincipals"',
  );
  const bucketConfigurationEnd = cacheModule.indexOf('Sid       = "ListCacheObjects"');
  assert.ok(bucketConfigurationStart >= 0);
  assert.ok(bucketConfigurationEnd > bucketConfigurationStart);
  const bucketConfigurationStatement = cacheModule.slice(
    bucketConfigurationStart,
    bucketConfigurationEnd,
  );
  assert.match(bucketConfigurationStatement, /Principal = "\*"/u);
  assert.match(bucketConfigurationStatement, /"s3:GetBucketCORS"/u);
  assert.match(bucketConfigurationStatement, /"s3:GetEncryptionConfiguration"/u);
  assert.match(bucketConfigurationStatement, /"s3:GetBucketObjectLockConfiguration"/u);
  assert.match(bucketConfigurationStatement, /"s3:GetLifecycleConfiguration"/u);
  assert.match(bucketConfigurationStatement, /"s3:PutEncryptionConfiguration"/u);
  assert.doesNotMatch(bucketConfigurationStatement, /s3:GetObject/u);
  assert.match(cacheModule, /resource "scaleway_iam_application" "branch_cache"/u);
  assert.match(
    cacheModule,
    /Resource {2}= \["\$\{scaleway_object_bucket\.ci_cache\.name\}\/trusted\/\*"\]/u,
  );
  assert.match(cacheModule, /Sid {7}= "ReadTrustedCacheObjectsFromBranches"/u);
  assert.match(
    cacheModule,
    /ReadTrustedCacheObjectsFromBranches[\s\S]*?Action {4}= \["s3:GetObject"\][\s\S]*?trusted\/\*/u,
  );
  const branchTrustedReadStart = cacheModule.indexOf(
    'Sid       = "ReadTrustedCacheObjectsFromBranches"',
  );
  const branchTrustedDenyStart = cacheModule.indexOf(
    'Sid       = "DenyTrustedCacheWritesFromBranches"',
  );
  assert.ok(branchTrustedReadStart >= 0);
  assert.ok(branchTrustedDenyStart > branchTrustedReadStart);
  assert.doesNotMatch(
    cacheModule.slice(branchTrustedReadStart, branchTrustedDenyStart),
    /s3:PutObject/u,
  );
  assert.match(
    cacheModule,
    /Resource {2}= \["\$\{scaleway_object_bucket\.ci_cache\.name\}\/branches\/\*"\]/u,
  );
  assert.doesNotMatch(
    cacheModule,
    /ReadWriteBranchCacheObjects[\s\S]*?ObjectStorageObjectsDelete/u,
  );
  assert.match(
    cacheModule,
    /DenyTrustedCacheWritesFromBranches[\s\S]*?Effect {4}= "Deny"[\s\S]*?"s3:DeleteObject", "s3:PutObject"[\s\S]*?trusted\/\*/u,
  );
});

test('branch pushes receive separate environment credentials', () => {
  assert.match(cacheModule, /resource "github_repository_environment" "branch_cache"/u);
  assert.match(cacheModule, /environment = var\.github_branch_environment/u);
  assert.match(cacheModule, /secret_name = "SCW_CI_CACHE_ACCESS_KEY"/u);
  assert.match(cacheModule, /secret_name = "SCW_CI_CACHE_SECRET_KEY"/u);
});

test('the security gate validates the branch cache trust boundary', () => {
  assert.match(securityChecker, /isValidatedBranchCacheWorkflow/u);
  assert.match(securityChecker, /text\.includes\('\[\[ "\$GITHUB_EVENT_NAME" == "push" \]\]'\)/u);
  assert.match(securityChecker, /text\.includes\('\[\[ "\$GITHUB_REF" == refs\/heads\/\* \]\]'\)/u);
  assert.match(securityChecker, /needs\.authorize-cache\.outputs\.trusted/u);
  assert.match(securityChecker, /node trusted-base\/tools\/ci\/authorize-pr-cache\.mjs/u);
});

test('cross-project bucket resources keep their explicit project scope', () => {
  assert.match(
    cacheModule,
    /resource "scaleway_object_bucket_server_side_encryption_configuration" "ci_cache" \{[\s\S]*?project_id = scaleway_account_project\.ci_cache\.id/u,
  );
  for (const application of ['ci_cache', 'branch_cache']) {
    assert.match(
      cacheModule,
      new RegExp(
        `resource "scaleway_iam_application" "${application}" \\{[\\s\\S]*?organization_id = var\\.organization_id`,
        'u',
      ),
    );
  }
  for (const policy of ['ci_cache', 'branch_cache']) {
    assert.match(
      cacheModule,
      new RegExp(
        `resource "scaleway_iam_policy" "${policy}" \\{[\\s\\S]*?organization_id = var\\.organization_id`,
        'u',
      ),
    );
  }
  assert.ok(bootstrapReadme.includes('fr-par/<bucket-name>@<project-id>'));
});

test('rotation is restricted to the protected bootstrap environment', () => {
  assert.match(rotationWorkflow, /if: github\.ref == 'refs\/heads\/main'/u);
  assert.match(rotationWorkflow, /name: production-bootstrap/u);
  assert.match(
    rotationWorkflow,
    /TF_VAR_terraform_state_bucket: \$\{\{ vars\.TERRAFORM_STATE_BUCKET \}\}/u,
  );
  assert.match(rotationWorkflow, /-target=module\.ci_cache/u);
  assert.match(rotationWorkflow, /ci-cache-rotation\.tfplan/u);
  assert.match(rotationWorkflow, /repair_bucket_policy:/u);
  assert.match(rotationWorkflow, /if: inputs\.repair_bucket_policy == true/u);
  assert.match(rotationWorkflow, /-refresh=false/u);
  assert.match(
    rotationWorkflow,
    /-target=module\.ci_cache\.scaleway_object_bucket_policy\.ci_cache/u,
  );
  assert.match(rotationWorkflow, /ci-cache-policy-repair\.tfplan/u);
});

test('CI lanes separate PR compilation writes from the protected S3 cache', () => {
  const setup = readFileSync('.github/actions/ci-setup/action.yml', 'utf8');
  assert.match(ciWorkflow, /needs\.authorize-cache\.outputs\.trusted == 'true'/u);
  assert.match(setup, /inputs\.rust == 'true'/u);
  assert.match(setup, /scaleway-cache-manager\.mjs restore/u);
  const credentials = {
    AWS_ACCESS_KEY_ID: 'fixture',
    AWS_SECRET_ACCESS_KEY: 'fixture',
    SCW_CI_CACHE_BUCKET: 'fixture',
    GITHUB_ACTIONS: 'true',
  };
  const main = compilationCacheEnvironment({
    ...credentials,
    GITHUB_EVENT_NAME: 'push',
    GITHUB_REF: 'refs/heads/main',
  });
  assert.equal(main.SCCACHE_S3_RW_MODE, 'READ_WRITE');
  assert.match(main.SCCACHE_S3_KEY_PREFIX, /^trusted\/rust\//u);
  assert.equal(main.SCCACHE_GHA_ENABLED, undefined);
  const pr = compilationCacheEnvironment({
    ...credentials,
    GITHUB_EVENT_NAME: 'pull_request',
    GITHUB_REF: 'refs/pull/243/merge',
    ACTIONS_RESULTS_URL: 'https://cache.invalid',
    ACTIONS_RUNTIME_TOKEN: 'fixture',
  });
  assert.equal(pr.SCCACHE_BUCKET, undefined);
  assert.equal(pr.AWS_SECRET_ACCESS_KEY, undefined);
  assert.equal(pr.SCCACHE_GHA_ENABLED, 'on');
  assert.equal(pr.SCCACHE_GHA_RW_MODE, 'READ_WRITE');
  assert.match(ciWorkflow, /success\(\) && github\.event_name == 'push'/u);
});
