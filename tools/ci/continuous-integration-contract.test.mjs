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
	assert.doesNotMatch(workflow, /Swatinem\/rust-cache@/u);
	assert.doesNotMatch(workflow, /^\s+cache: pnpm\s*$/mu);
	assert.match(
		workflow,
		/node --test tools\/ci\/continuous-integration-contract\.test\.mjs/u,
	);
});
