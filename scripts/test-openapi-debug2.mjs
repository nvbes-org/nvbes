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

function handleRef(ref, item, spec) {
	if (item && typeof item === "object" && "$ref" in item) {
		const segments = item.$ref.slice(2).split("/");
		let current = spec;
		for (const seg of segments) current = current?.[seg];
		return current;
	}
	return item;
}

function isProtectedOperation(operation, globalSecurity) {
	if (Array.isArray(operation.security)) return operation.security.length > 0;
	return Array.isArray(globalSecurity) && globalSecurity.length > 0;
}

function shouldSkip(pathPattern, method, operation) {
	const key = `${method.toUpperCase()} ${pathPattern}`;
	if (ALWAYS_SKIP_OPERATIONS.has(key)) return true;
	if (ALWAYS_SKIP_PATH_SUBSTRINGS.some((s) => pathPattern.includes(s)))
		return true;
	const protectedOp = isProtectedOperation(operation, globalSecurity);
	if (protectedOp) return true;
	if (!protectedOp && MUTATING_METHODS.has(method) && pathPattern.includes("{"))
		return true;
	return false;
}

function sampleParam(parameter) {
	const name = parameter.name?.toLowerCase() ?? "";
	if (name.endsWith("id") || name.includes("uuid"))
		return "00000000-0000-0000-0000-000000000000";
	if (
		name.includes("page") ||
		name.includes("limit") ||
		name.includes("offset")
	)
		return "1";
	return "smoke";
}

function buildPath(pathPattern, parameters) {
	let result = pathPattern;
	for (const param of parameters) {
		if (param.in === "path") {
			result = result.replace(
				`{${param.name}}`,
				encodeURIComponent(sampleParam(param)),
			);
		}
	}
	const queryParams = parameters.filter((p) => p.in === "query");
	if (queryParams.length > 0) {
		const sp = new URLSearchParams();
		for (const p of queryParams) sp.set(p.name, sampleParam(p));
		result += "?" + sp.toString();
	}
	return result;
}

let count = 0;
const apiBaseUrl = "http://localhost:4000";
const webBaseUrl = "http://localhost:3001";

for (const [pathPattern, pathItem] of Object.entries(paths)) {
	for (const [method, rawOperation] of Object.entries(pathItem)) {
		if (!ROUTABLE_METHODS.includes(method)) continue;

		const operation = handleRef("$ref", rawOperation, spec) ?? rawOperation;
		if (shouldSkip(pathPattern, method, operation)) continue;

		const parameters = [];
		for (const p of [
			...(pathItem.parameters ?? []),
			...(operation.parameters ?? []),
		]) {
			const param = handleRef("$ref", p, spec) ?? p;
			if (param) parameters.push(param);
		}
		const resolvedPath = buildPath(pathPattern, parameters);

		count++;
		const url = `${apiBaseUrl}${resolvedPath}`;
		const headers = { Accept: "*/*" };
		let body;
		if (MUTATING_METHODS.has(method)) {
			headers.Origin = "http://localhost:3001";
			headers["Content-Type"] = "application/json";
			body = "{}";
		}

		try {
			const resp = await fetch(url, {
				method: method.toUpperCase(),
				headers,
				body,
			});
			if (resp.status >= 500) {
				const text = await resp.text().catch(() => "");
				console.log(
					`REQUEST ${count}: ${method.toUpperCase()} ${url} -> ${resp.status} 5xx`,
				);
				console.log(text.substring(0, 200));
				process.exit(1);
			}
		} catch (err) {
			console.log(`REQUEST ${count}: ${method.toUpperCase()} ${url}`);
			console.log(
				`FAILED: ${err.message}, cause: ${err.cause?.code ?? "none"}`,
			);
			process.exit(1);
		}
	}
}
console.log(`All ${count} API requests passed`);
