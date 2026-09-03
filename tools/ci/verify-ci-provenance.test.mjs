import assert from "node:assert/strict";
import test from "node:test";
import {
	successfulContinuousIntegrationRun,
	validateDeploymentRevision,
} from "./verify-ci-provenance.core.mjs";

const validRun = {
	id: 42,
	repository: { full_name: "nvbes-org/nvbes" },
	path: ".github/workflows/ci.yml",
	event: "push",
	head_branch: "main",
	head_sha: "a".repeat(40),
	status: "completed",
	conclusion: "success",
};

test("accepts only the successful main CI push for the exact repository and SHA", () => {
	assert.equal(
		successfulContinuousIntegrationRun([validRun], {
			repository: "nvbes-org/nvbes",
			sha: "a".repeat(40),
		}),
		validRun,
	);
	for (const override of [
		{ event: "pull_request" },
		{ head_branch: "dev" },
		{ head_sha: "b".repeat(40) },
		{ conclusion: "failure" },
		{ path: ".github/workflows/other.yml" },
	]) {
		assert.equal(
			successfulContinuousIntegrationRun([{ ...validRun, ...override }], {
				repository: "nvbes-org/nvbes",
				sha: "a".repeat(40),
			}),
			undefined,
		);
	}
});

test("production provenance accepts only a full main commit", () => {
	assert.doesNotThrow(() =>
		validateDeploymentRevision({ ref: "refs/heads/main", sha: "a".repeat(40) }),
	);
	assert.throws(() =>
		validateDeploymentRevision({ ref: "refs/heads/dev", sha: "a".repeat(40) }),
	);
	assert.throws(() =>
		validateDeploymentRevision({ ref: "refs/heads/main", sha: "main" }),
	);
});
