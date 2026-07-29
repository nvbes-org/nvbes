import assert from "node:assert/strict";
import test from "node:test";
import { verifyAccountReleaseRef } from "./verify-account-release-ref.mjs";

const releaseSha = "a".repeat(40);
const tag = `account-rc-${releaseSha}`;
const config = {
	apiUrl: "https://api.github.com",
	ref: `refs/tags/${tag}`,
	refName: tag,
	refType: "tag",
	releaseSha,
	repository: "nvbes/nvbes",
	token: "test-token",
};

test("accepts only a lightweight RC tag resolving to the exact release SHA", async () => {
	const calls = [];
	const result = await verifyAccountReleaseRef(config, {
		fetchImpl: async (url, options) => {
			calls.push({ options, url });
			return jsonResponse({
				object: { sha: releaseSha, type: "commit" },
				ref: `refs/tags/${tag}`,
			});
		},
	});
	assert.deepEqual(result, { releaseSha, tag });
	assert.equal(
		calls[0].url,
		`https://api.github.com/repos/nvbes/nvbes/git/ref/tags/${tag}`,
	);
	assert.equal(calls[0].options.redirect, "error");
});

test("rejects branches and tag names that do not bind the release SHA", async () => {
	for (const override of [
		{ refType: "branch" },
		{ refName: "main" },
		{ ref: "refs/tags/account-rc-latest" },
	]) {
		await assert.rejects(
			verifyAccountReleaseRef(
				{ ...config, ...override },
				{
					fetchImpl: async () => {
						throw new Error("must not fetch");
					},
				},
			),
			/RC tag/u,
		);
	}
});

test("rejects annotated or moved RC tags", async () => {
	for (const reference of [
		{ object: { sha: releaseSha, type: "tag" }, ref: `refs/tags/${tag}` },
		{
			object: { sha: "b".repeat(40), type: "commit" },
			ref: `refs/tags/${tag}`,
		},
		{ object: { sha: releaseSha, type: "commit" }, ref: "refs/tags/other" },
	]) {
		await assert.rejects(
			verifyAccountReleaseRef(config, {
				fetchImpl: async () => jsonResponse(reference),
			}),
			/annotated, moved/u,
		);
	}
});

function jsonResponse(value) {
	return new Response(JSON.stringify(value), {
		headers: { "content-type": "application/json" },
		status: 200,
	});
}
