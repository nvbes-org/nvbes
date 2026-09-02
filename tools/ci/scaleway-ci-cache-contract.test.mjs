import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const emailWorkflow = readFileSync(
	".github/workflows/deploy-email.yml",
	"utf8",
);
const trustRiskWorkflow = readFileSync(
	".github/workflows/deploy-trust-risk.yml",
	"utf8",
);
const rotationWorkflow = readFileSync(
	".github/workflows/rotate-ci-cache-credentials.yml",
	"utf8",
);
const ciWorkflow = readFileSync(".github/workflows/ci.yml", "utf8");
const cacheModule = readFileSync(
	"infrastructure/modules/scaleway-ci-cache/main.tf",
	"utf8",
);
const bootstrapReadme = readFileSync(
	"infrastructure/bootstrap/production/README.md",
	"utf8",
);
const securityChecker = readFileSync(
	"tools/security/check-ci-cd-security.mjs",
	"utf8",
);

test("deployment builds use isolated Scaleway registry cache scopes", () => {
	for (const [workflow, scope] of [
		[emailWorkflow, "email-worker"],
		[trustRiskWorkflow, "trust-risk-service"],
	]) {
		assert.match(workflow, /name: production-ci-cache/u);
		assert.match(
			workflow,
			/password: \$\{\{ secrets\.SCW_CI_CACHE_SECRET_KEY \}\}/u,
		);
		assert.ok(
			workflow.includes(
				`cache-from: type=registry,ref=\${{ env.CI_CACHE_REGISTRY }}/${scope}:buildcache`,
			),
		);
		assert.ok(
			workflow.includes(
				`cache-to: type=registry,ref=\${{ env.CI_CACHE_REGISTRY }}/${scope}:buildcache,mode=max,compression=zstd,oci-mediatypes=true,image-manifest=true`,
			),
		);
	}
});

test("cache credentials rotate through two staggered slots", () => {
	assert.match(cacheModule, /credential_slots = \{/u);
	assert.match(cacheModule, /b = floor\(var\.credential_rotation_days \/ 2\)/u);
	assert.match(cacheModule, /active_credential_slot = timecmp\(/u);
	assert.match(cacheModule, /for_each = local\.credential_slots/u);
	assert.match(cacheModule, /create_before_destroy = true/u);
	assert.match(cacheModule, /scaleway_account_project\.ci_cache\.id/u);
	assert.match(cacheModule, /branch_pattern = "main"/u);
	assert.match(cacheModule, /branch_pattern = "dev"/u);
	assert.match(cacheModule, /branch_pattern = "staging"/u);
	assert.match(cacheModule, /branch_pattern = "release\/\*"/u);
	assert.match(cacheModule, /resource "time_rotating" "branch_cache"/u);
	assert.match(cacheModule, /resource "scaleway_iam_api_key" "branch_cache"/u);
});

test("cache access is restricted to the CI application and dedicated bucket", () => {
	assert.match(
		cacheModule,
		/resource "scaleway_object_bucket_policy" "ci_cache"/u,
	);
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
	const bucketConfigurationEnd = cacheModule.indexOf(
		'Sid       = "ListCacheObjects"',
	);
	assert.ok(bucketConfigurationStart >= 0);
	assert.ok(bucketConfigurationEnd > bucketConfigurationStart);
	const bucketConfigurationStatement = cacheModule.slice(
		bucketConfigurationStart,
		bucketConfigurationEnd,
	);
	assert.match(bucketConfigurationStatement, /Principal = "\*"/u);
	assert.match(bucketConfigurationStatement, /"s3:GetBucketCORS"/u);
	assert.match(
		bucketConfigurationStatement,
		/"s3:GetBucketObjectLockConfiguration"/u,
	);
	assert.match(bucketConfigurationStatement, /"s3:GetLifecycleConfiguration"/u);
	assert.doesNotMatch(bucketConfigurationStatement, /s3:GetObject/u);
	assert.match(
		cacheModule,
		/resource "scaleway_iam_application" "branch_cache"/u,
	);
	assert.match(
		cacheModule,
		/Resource {2}= \["\$\{scaleway_object_bucket\.ci_cache\.name\}\/trusted\/\*"\]/u,
	);
	assert.match(
		cacheModule,
		/Resource {2}= \["\$\{scaleway_object_bucket\.ci_cache\.name\}\/branches\/\*"\]/u,
	);
	assert.doesNotMatch(
		cacheModule,
		/ReadWriteBranchCacheObjects[\s\S]*?ObjectStorageObjectsDelete/u,
	);
});

test("branch pushes receive separate environment credentials", () => {
	assert.match(
		cacheModule,
		/resource "github_repository_environment" "branch_cache"/u,
	);
	assert.match(cacheModule, /environment = var\.github_branch_environment/u);
	assert.match(cacheModule, /secret_name = "SCW_CI_CACHE_ACCESS_KEY"/u);
	assert.match(cacheModule, /secret_name = "SCW_CI_CACHE_SECRET_KEY"/u);
});

test("the security gate validates the branch cache trust boundary", () => {
	assert.match(securityChecker, /isValidatedBranchCacheWorkflow/u);
	assert.match(
		securityChecker,
		/text\.includes\('\[\[ "\$GITHUB_EVENT_NAME" == "push" \]\]'\)/u,
	);
	assert.match(
		securityChecker,
		/text\.includes\('\[\[ "\$GITHUB_REF" == refs\/heads\/\* \]\]'\)/u,
	);
	assert.match(
		securityChecker,
		/'production-ci-cache' \|\| 'branch-ci-cache'/u,
	);
	assert.match(
		securityChecker,
		/format\('branches\/\{0\}', github\.ref_name\)/u,
	);
});

test("cross-project bucket resources keep their explicit project scope", () => {
	assert.match(
		cacheModule,
		/resource "scaleway_object_bucket_server_side_encryption_configuration" "ci_cache" \{[\s\S]*?project_id = scaleway_account_project\.ci_cache\.id/u,
	);
	for (const application of ["ci_cache", "branch_cache"]) {
		assert.match(
			cacheModule,
			new RegExp(
				`resource "scaleway_iam_application" "${application}" \\{[\\s\\S]*?organization_id = var\\.organization_id`,
				"u",
			),
		);
	}
	for (const policy of ["ci_cache", "branch_cache"]) {
		assert.match(
			cacheModule,
			new RegExp(
				`resource "scaleway_iam_policy" "${policy}" \\{[\\s\\S]*?organization_id = var\\.organization_id`,
				"u",
			),
		);
	}
	assert.ok(bootstrapReadme.includes("fr-par/<bucket-name>@<project-id>"));
});

test("rotation is restricted to the protected bootstrap environment", () => {
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
	assert.match(rotationWorkflow, /migrate_bucket_encryption:/u);
	assert.match(
		rotationWorkflow,
		/if: inputs\.migrate_bucket_encryption == true/u,
	);
	assert.match(rotationWorkflow, /-destroy/u);
	assert.match(
		rotationWorkflow,
		/-target=module\.ci_cache\.scaleway_object_bucket_policy\.ci_cache/u,
	);
	assert.match(rotationWorkflow, /ci-cache-policy-removal\.tfplan/u);
	assert.match(
		cacheModule,
		/depends_on = \[scaleway_object_bucket_server_side_encryption_configuration\.ci_cache\]/u,
	);
});

test("branch pushes use the correctly scoped Scaleway S3 cache", () => {
	assert.match(
		ciWorkflow,
		/rust-tests-scaleway-cache:\n {4}if: github\.event_name == 'push'/u,
	);
	assert.match(ciWorkflow, /'production-ci-cache' \|\| 'branch-ci-cache'/u);
	assert.match(ciWorkflow, /RUSTC_WRAPPER: sccache/u);
	assert.match(ciWorkflow, /CARGO_INCREMENTAL: '0'/u);
	assert.match(
		ciWorkflow,
		/SCCACHE_BUCKET: \$\{\{ vars\.SCW_CI_CACHE_BUCKET \}\}/u,
	);
	assert.match(
		ciWorkflow,
		/SCCACHE_ENDPOINT: \$\{\{ vars\.SCW_CI_CACHE_S3_ENDPOINT \}\}/u,
	);
	assert.match(
		ciWorkflow,
		/SCCACHE_REGION: \$\{\{ vars\.SCW_CI_CACHE_REGION \}\}/u,
	);
	assert.match(
		ciWorkflow,
		/SCCACHE_S3_KEY_PREFIX: \$\{\{[\s\S]*?\}\}\/rust\/\$\{\{ runner\.os \}\}-\$\{\{ runner\.arch \}\}\/rust-1\.91\.1/u,
	);
	assert.match(ciWorkflow, /format\('branches\/\{0\}', github\.ref_name\)/u);
	assert.match(ciWorkflow, /name: Detect Scaleway cache credentials/u);
	assert.match(
		ciWorkflow,
		/name: Run Rust tests with Scaleway cache\n {8}if: github\.event_name == 'push' && steps\.scaleway-cache\.outputs\.available == 'true'/u,
	);
	assert.match(
		ciWorkflow,
		/name: Run Rust tests with bootstrap cache fallback\n {8}if: github\.event_name == 'push' && steps\.scaleway-cache\.outputs\.available != 'true'[\s\S]*?SCCACHE_GHA_ENABLED: 'true'/u,
	);
	assert.match(
		ciWorkflow,
		/AWS_ACCESS_KEY_ID: \$\{\{ secrets\.SCW_CI_CACHE_ACCESS_KEY \}\}/u,
	);
	assert.match(
		ciWorkflow,
		/AWS_SECRET_ACCESS_KEY: \$\{\{ secrets\.SCW_CI_CACHE_SECRET_KEY \}\}/u,
	);
	assert.match(
		ciWorkflow,
		/sccache --zero-stats\n {10}cargo test --workspace --locked\n {10}sccache --show-stats/u,
	);
	assert.match(
		ciWorkflow,
		/name: Test email contract and runtime\n {8}if: github\.event_name != 'push'\n {8}run: pnpm test/u,
	);
});
