import assert from "node:assert/strict";
import test from "node:test";
import {
	branchCacheId,
	isTrustedRef,
	restorePrefixes,
	writablePrefix,
} from "./scaleway-cache-manager.core.mjs";

test("protected refs share the trusted namespace", () => {
	for (const ref of [
		"refs/heads/main",
		"refs/heads/dev",
		"refs/heads/release/v1",
	]) {
		assert.equal(isTrustedRef(ref), true);
		assert.deepEqual(restorePrefixes({ ref }), ["trusted"]);
		assert.equal(writablePrefix({ eventName: "push", ref }), "trusted");
	}
});

test("branches restore their isolated cache before the trusted cache", () => {
	const environment = {
		eventName: "push",
		ref: "refs/heads/feature/cache",
		refName: "feature/cache",
	};
	const branch = `branches/${branchCacheId(environment)}`;
	assert.deepEqual(restorePrefixes(environment), [branch, "trusted"]);
	assert.equal(writablePrefix(environment), branch);
});

test("pull requests never publish archives and use the head branch identity", () => {
	const environment = {
		eventName: "pull_request",
		ref: "refs/pull/42/merge",
		headRef: "feature/cache",
	};
	assert.equal(writablePrefix(environment), null);
	assert.equal(
		restorePrefixes(environment)[0],
		`branches/${branchCacheId(environment)}`,
	);
});
