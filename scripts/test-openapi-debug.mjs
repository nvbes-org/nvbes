#!/usr/bin/env node

import { execFileSync } from "node:child_process";

const ROOT_DIR = "/Users/shayn/Development/nvbes";
const PROXIED_PREFIXES = ["/api", "/auth", "/oauth", "/csp-report"];
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
	["run", "-p", "nvbes-identity-api", "--", "--export-openapi"],
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

function isProtectedOperation(operation, globalSecurity) {
	if (Array.isArray(operation.security)) {
		return operation.security.length > 0;
	}
	return Array.isArray(globalSecurity) && globalSecurity.length > 0;
}

let lastKey = "";
for (const [pathPattern, pathItem] of Object.entries(paths)) {
	for (const [method, rawOperation] of Object.entries(pathItem)) {
		if (!ROUTABLE_METHODS.includes(method)) continue;

		const key = `${method.toUpperCase()} ${pathPattern}`;

		if (ALWAYS_SKIP_OPERATIONS.has(key)) continue;
		if (
			ALWAYS_SKIP_PATH_SUBSTRINGS.some((segment) =>
				pathPattern.includes(segment),
			)
		)
			continue;

		const operation = rawOperation;
		const protectedOp = isProtectedOperation(operation, globalSecurity);
		if (protectedOp) continue;
		if (
			!protectedOp &&
			MUTATING_METHODS.has(method) &&
			pathPattern.includes("{")
		)
			continue;

		lastKey = key;

		const url = `http://localhost:4000${pathPattern}`;
		const headers = { Accept: "*/*" };
		const body = MUTATING_METHODS.has(method)
			? ((headers["Content-Type"] = "application/json"),
				(headers.Origin = "http://localhost:3001"),
				"{}")
			: undefined;

		try {
			const resp = await fetch(url, {
				method: method.toUpperCase(),
				headers,
				body,
			});
			if (resp.status >= 500) {
				const text = await resp.text().catch(() => "");
				console.log(`5xx: ${key} ${resp.status} ${text.substring(0, 100)}`);
				process.exit(1);
			} else if (resp.status === 0 || resp.status === null) {
				console.log(`BAD: ${key} status=${resp.status}`);
				process.exit(1);
			} else {
				// OK
			}
		} catch (err) {
			console.log(`FAIL: ${key}`);
			console.log(`error: ${err.message}`);
			console.log(`cause: ${JSON.stringify(err.cause)}`);
			console.log(`last successful: ${lastKey}`);
			process.exit(1);
		}
	}
}
console.log("All passed");
