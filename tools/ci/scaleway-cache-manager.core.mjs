import { createHash } from "node:crypto";

const protectedRefs = new Set(["refs/heads/main", "refs/heads/dev"]);

export function isTrustedRef(ref) {
	return protectedRefs.has(ref) || ref.startsWith("refs/heads/release/");
}

export function branchCacheId({ headRef = "", refName = "", ref = "" }) {
	return createHash("sha256")
		.update(headRef || refName || ref)
		.digest("hex")
		.slice(0, 20);
}

export function restorePrefixes(environment) {
	if (isTrustedRef(environment.ref)) return ["trusted"];
	return [`branches/${branchCacheId(environment)}`, "trusted"];
}

export function writablePrefix(environment) {
	if (environment.eventName !== "push") return null;
	return isTrustedRef(environment.ref)
		? "trusted"
		: `branches/${branchCacheId(environment)}`;
}
