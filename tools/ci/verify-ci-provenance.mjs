#!/usr/bin/env node
import {
	successfulContinuousIntegrationRun,
	validateDeploymentRevision,
} from "./verify-ci-provenance.core.mjs";

const {
	GITHUB_API_URL,
	GITHUB_REF,
	GITHUB_REPOSITORY,
	GITHUB_SHA,
	GITHUB_TOKEN,
} = process.env;

validateDeploymentRevision({ ref: GITHUB_REF, sha: GITHUB_SHA });
if (!GITHUB_REPOSITORY || !GITHUB_TOKEN) {
	throw new Error("GITHUB_REPOSITORY and GITHUB_TOKEN are required");
}

const apiUrl = GITHUB_API_URL || "https://api.github.com";
const query = new URLSearchParams({
	branch: "main",
	event: "push",
	head_sha: GITHUB_SHA,
	per_page: "10",
	status: "success",
});
const response = await fetch(
	`${apiUrl}/repos/${GITHUB_REPOSITORY}/actions/workflows/ci.yml/runs?${query}`,
	{
		headers: {
			Accept: "application/vnd.github+json",
			Authorization: `Bearer ${GITHUB_TOKEN}`,
			"X-GitHub-Api-Version": "2022-11-28",
		},
	},
);
if (!response.ok) {
	throw new Error(
		`GitHub Actions provenance lookup failed with ${response.status}`,
	);
}

const payload = await response.json();
const run = successfulContinuousIntegrationRun(payload.workflow_runs ?? [], {
	repository: GITHUB_REPOSITORY,
	sha: GITHUB_SHA,
});
if (!run) {
	throw new Error(
		`No successful continuous integration push run found for ${GITHUB_SHA}`,
	);
}

console.log(`Verified continuous integration run ${run.id} for ${GITHUB_SHA}`);
