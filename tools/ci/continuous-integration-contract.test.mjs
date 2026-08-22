import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const workflow = readFileSync(".github/workflows/ci.yml", "utf8");
const unitRunner = readFileSync("scripts/test-unit.sh", "utf8");

test("continuous CI keeps bounded unit-test parallelism", () => {
	assert.match(unitRunner, /--test-threads=4/u);
	assert.match(unitRunner, /--exclude nvbes-identity-worker/u);
	assert.match(
		unitRunner,
		/--skip postgresql_fresh_database_reaches_complete_schema/u,
	);
	assert.match(
		unitRunner,
		/--skip postgresql_upgrade_from_n_minus_one_preserves_compatible_data/u,
	);
	assert.doesNotMatch(
		unitRunner,
		/cargo test --workspace[^\n]*--test-threads=1/u,
	);
	assert.match(
		unitRunner,
		/cargo test --package nvbes-identity-worker --locked -- --test-threads=1/u,
	);
});

test("continuous CI isolates migration scenarios and avoids remote cache stalls", () => {
	assert.match(workflow, /identity-migration-checks:/u);
	assert.match(workflow, /bash scripts\/test-identity-service-migrations\.sh/u);
	assert.match(workflow, /job\.services\.postgres\.ports\[5432\]/u);
	assert.match(workflow, /job\.services\.redis\.ports\[6379\]/u);
	assert.match(workflow, /name: Export test service URLs/u);
	assert.match(workflow, /host\.docker\.internal/u);
	assert.doesNotMatch(workflow, /Pre-build: Setup test services/u);
	assert.doesNotMatch(workflow, /^\s+- 5432:5432\s*$/mu);
	assert.doesNotMatch(workflow, /^\s+- 6379:6379\s*$/mu);
	assert.doesNotMatch(workflow, /Swatinem\/rust-cache@/u);
	assert.doesNotMatch(workflow, /^\s+cache: pnpm\s*$/mu);
	assert.match(
		workflow,
		/node --test tools\/ci\/continuous-integration-contract\.test\.mjs/u,
	);
	assert.match(unitRunner, /GITHUB_ACTIONS:-/u);
});
