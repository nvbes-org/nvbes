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
const cacheModule = readFileSync(
	"infrastructure/modules/scaleway-ci-cache/main.tf",
	"utf8",
);
const bootstrapReadme = readFileSync(
	"infrastructure/bootstrap/production/README.md",
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
});

test("cross-project bucket resources keep their explicit project scope", () => {
	assert.match(
		cacheModule,
		/resource "scaleway_object_bucket_server_side_encryption_configuration" "ci_cache" \{[\s\S]*?project_id = scaleway_account_project\.ci_cache\.id/u,
	);
	assert.ok(
		bootstrapReadme.includes(
			"fr-par/<bucket-name>@<project-id>",
		),
	);
});

test("rotation is restricted to the protected bootstrap environment", () => {
	assert.match(rotationWorkflow, /if: github\.ref == 'refs\/heads\/main'/u);
	assert.match(rotationWorkflow, /name: production-bootstrap/u);
	assert.match(rotationWorkflow, /-target=module\.ci_cache/u);
	assert.match(rotationWorkflow, /ci-cache-rotation\.tfplan/u);
});
