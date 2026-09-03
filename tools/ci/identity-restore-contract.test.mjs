import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(
	".github/workflows/validate-identity-restore.yml",
	"utf8",
);

test("Identity restore is restricted to an exact approved main revision", () => {
	assert.match(workflow, /if: github\.ref == 'refs\/heads\/main'/u);
	assert.match(workflow, /ref: \$\{\{ inputs\.approved_sha \}\}/u);
	assert.match(workflow, /\[\[ "\$APPROVED_SHA" =~ \^\[0-9a-f\]\{40\}\$ \]\]/u);
	assert.match(
		workflow,
		/\[\[ "\$APPROVED_SHA" == "\$\(git rev-parse HEAD\)" \]\]/u,
	);
	assert.match(workflow, /validate-identity-production-restore/u);
});

test("Identity restore is isolated, bounded, verified and automatically removed", () => {
	assert.match(
		workflow,
		/- name: Provision isolated restore[\s\S]*?AWS_ACCESS_KEY_ID: \$\{\{ secrets\.IDENTITY_TERRAFORM_STATE_ACCESS_KEY \}\}[\s\S]*?terraform -chdir=infrastructure\/environments\/identity-production output/u,
	);
	assert.match(workflow, /from_backup_id: \$backup_id/u);
	assert.match(workflow, /cpu_min: 0, cpu_max: 1/u);
	assert.match(workflow, /\.migrations == 3 and \.tables == 8/u);
	assert.match(workflow, /\.principal == 1 and \.audit_events > 0/u);
	assert.match(workflow, /--file=- <<'SQL'/u);
	assert.doesNotMatch(workflow, /--command "SELECT/u);
	assert.match(workflow, /DELETE_RESTORE_DATABASE=true/u);
	assert.match(workflow, /if: \$\{\{ always\(\) \}\}/u);
});

test("Identity recovery evidence contains no identity data", () => {
	assert.match(
		workflow,
		/safe_integrity=.*del\(\.principal, \.audit_events\)/u,
	);
	assert.doesNotMatch(workflow, /SELECT \*/u);
	assert.doesNotMatch(workflow, /normalized_value/u);
	assert.doesNotMatch(workflow, /password_hash/u);
});
