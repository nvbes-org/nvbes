#!/usr/bin/env node

import { execFileSync } from "node:child_process";

const ROOT_DIR = "/Users/shayn/Development/nvbes";
const MUTATING_METHODS = new Set(["post", "put", "patch", "delete"]);
const ROUTABLE_METHODS = [
	"get",
	"post",
	"put",
	"patch",
	"delete",
	"head",
	"options",
];
const ALWAYS_SKIP_PATH_SUBSTRINGS = [
	"/webhooks/",
	"/oauth/callback",
	"/saml/acs",
];
const ALWAYS_SKIP_OPERATIONS = new Set([
	"POST /auth/logout",
	"DELETE /auth/sessions/{sessionId}",
	"POST /auth/sessions/revoke-others",
	"POST /auth/me/delete",
]);

const stdout = execFileSync(
	"cargo",
	["run", "-p", "nvbes-account-service", "--", "--export-openapi"],
	{
		cwd: ROOT_DIR,
		encoding: "utf8",
		maxBuffer: 20 * 1024 * 1024,
		env: process.env,
	},
);
const spec = JSON.parse(stdout);
const paths = spec.paths ?? {};
const globalSecurity = spec.security;

function isProtectedOp(op, gs) {
	if (Array.isArray(op.security)) return op.security.length > 0;
	return Array.isArray(gs) && gs.length > 0;
}

let directCount = 0;
let proxyCount = 0;

for (const [pathPattern, pathItem] of Object.entries(paths)) {
	for (const [method, rawOp] of Object.entries(pathItem)) {
		if (!ROUTABLE_METHODS.includes(method)) continue;
		const key = `${method.toUpperCase()} ${pathPattern}`;
		if (ALWAYS_SKIP_OPERATIONS.has(key)) continue;
		if (ALWAYS_SKIP_PATH_SUBSTRINGS.some((s) => pathPattern.includes(s)))
			continue;

		const op = rawOp;
		const protectedOp = isProtectedOp(op, globalSecurity);
		if (protectedOp) continue;
		if (
			!protectedOp &&
			MUTATING_METHODS.has(method) &&
			pathPattern.includes("{")
		)
			continue;

		const shouldProxy = ["/api", "/auth", "/oauth", "/csp-report"].some((p) =>
			pathPattern.startsWith(p),
		);
		directCount++;
		if (shouldProxy) proxyCount++;
	}
}

console.log(`Non-skipped: ${directCount} direct, ${proxyCount} proxy`);
console.log(`Total iterations: ${directCount + proxyCount}`);
