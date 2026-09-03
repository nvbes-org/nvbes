import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const deployableWorkflows = [
	"account",
	"billing",
	"email",
	"identity",
	"trust-risk",
];

for (const service of deployableWorkflows) {
	const path = `.github/workflows/deploy-${service}.yml`;
	const workflow = readFileSync(path, "utf8");
	test(`${service} deployment promotes only an exactly validated main revision`, () => {
		assert.match(workflow, /^  ci-provenance:\n/mu);
		assert.match(workflow, /node tools\/ci\/verify-ci-provenance\.mjs/u);
		assert.match(workflow, /GITHUB_TOKEN: \$\{\{ github\.token \}\}/u);
		assert.match(workflow, /needs: \[ci-provenance\]/u);
		assert.doesNotMatch(workflow, /^  ci-test-gate:/mu);
	});

	test(`${service} deployment does not rebuild its CI gate`, () => {
		const gate = workflow.slice(
			workflow.indexOf("  ci-provenance:"),
			workflow.indexOf("  build-scan-sign:"),
		);
		assert.doesNotMatch(
			gate,
			/pnpm install|cargo (?:check|test)|terraform .*validate/u,
		);
	});
}
