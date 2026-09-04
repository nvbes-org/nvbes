#!/usr/bin/env node

import { appendFileSync, readFileSync } from "node:fs";
import { evaluatePullRequestCacheTrust } from "./authorize-pr-cache.core.mjs";

function requiredEnvironment(name) {
	const value = process.env[name];
	if (!value) throw new Error(`${name} must be set`);
	return value;
}

async function listPullRequestResource(apiUrl, repository, number, resource) {
	const token = requiredEnvironment("GITHUB_TOKEN");
	const values = [];
	for (let page = 1; page <= 10; page += 1) {
		const response = await fetch(
			`${apiUrl}/repos/${repository}/pulls/${number}/${resource}?per_page=100&page=${page}`,
			{
				headers: {
					Accept: "application/vnd.github+json",
					Authorization: `Bearer ${token}`,
					"X-GitHub-Api-Version": "2022-11-28",
				},
			},
		);
		if (!response.ok) {
			throw new Error(
				`GitHub ${resource} query failed with ${response.status}`,
			);
		}
		const pageValues = await response.json();
		if (!Array.isArray(pageValues))
			throw new Error(`Invalid ${resource} response`);
		values.push(...pageValues);
		if (pageValues.length < 100) return values;
	}
	throw new Error(`Pull request ${resource} exceed the authorization limit`);
}

const event = JSON.parse(
	readFileSync(requiredEnvironment("GITHUB_EVENT_PATH"), "utf8"),
);
const repository = requiredEnvironment("GITHUB_REPOSITORY");
const apiUrl = requiredEnvironment("GITHUB_API_URL");
const output = requiredEnvironment("GITHUB_OUTPUT");
let result = { trusted: false, reasons: ["authorization failed closed"] };

try {
	const number = event.pull_request?.number;
	if (!Number.isInteger(number))
		throw new Error("Pull request number is missing");
	const [commits, files] = await Promise.all([
		listPullRequestResource(apiUrl, repository, number, "commits"),
		listPullRequestResource(apiUrl, repository, number, "files"),
	]);
	result = evaluatePullRequestCacheTrust(event, commits, files);
} catch (error) {
	result = { trusted: false, reasons: [error.message] };
}

appendFileSync(output, `trusted=${result.trusted}\n`);
console.log(
	result.trusted
		? "Internal pull request cache access authorized"
		: `Central cache disabled: ${result.reasons.join("; ")}`,
);
