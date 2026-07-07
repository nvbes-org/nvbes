#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT_DIR = path.resolve(__dirname, "..");
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
const JSON_CONTENT_TYPES = new Set([
	"application/json",
	"application/merge-patch+json",
	"application/ld+json",
]);
const SAFE_PROTECTED_MUTATING_OPERATIONS = new Set([
	"PATCH /auth/me",
	"PUT /auth/me/preferences",
	"PUT /auth/me/notifications",
	"POST /authz/decision",
]);
const ALWAYS_SKIP_OPERATIONS = new Set([
	"POST /auth/logout",
	"DELETE /auth/sessions/{sessionId}",
	"POST /auth/sessions/revoke-others",
	"POST /auth/me/delete",
]);
const ALWAYS_SKIP_PATH_SUBSTRINGS = [
	"/webhooks/",
	"/oauth/callback",
	"/saml/acs",
	"/auth/challenge/",
	"/auth/password/",
	"/auth/verify-email",
	"/auth/register",
];
const SEEDED_AUTH_ENABLED = readEnvBoolean("NVBES_SMOKE_CONTRACT_SEEDED_AUTH");

function fail(message) {
	throw new Error(message);
}

function readEnvBoolean(name) {
	const value = process.env[name];
	if (!value) {
		return false;
	}
	return ["1", "true", "yes", "on"].includes(value.toLowerCase());
}

function normalizeUrl(value, name) {
	if (!value) {
		fail(`missing required environment variable: ${name}`);
	}
	return value.replace(/\/+$/, "");
}

function loadOpenApiSpec() {
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

	return JSON.parse(stdout);
}

function resolveRef(ref, spec) {
	if (typeof ref !== "string" || !ref.startsWith("#/")) {
		return undefined;
	}

	const segments = ref.slice(2).split("/");
	let current = spec;
	for (const segment of segments) {
		current = current?.[segment];
	}
	return current;
}

function deref(value, spec) {
	if (value && typeof value === "object" && "$ref" in value) {
		const resolved = resolveRef(value.$ref, spec);
		if (!resolved) {
			fail(`unable to resolve OpenAPI ref: ${value.$ref}`);
		}
		return resolved;
	}

	return value;
}

function operationKey(method, pathPattern) {
	return `${method.toUpperCase()} ${pathPattern}`;
}

function collectParameters(pathItem, operation, spec) {
	const parameters = [];
	for (const entry of [
		...(pathItem.parameters ?? []),
		...(operation.parameters ?? []),
	]) {
		const parameter = deref(entry, spec);
		if (parameter) {
			parameters.push(parameter);
		}
	}
	return parameters;
}

function sampleFromSchema(schema, spec) {
	const resolved = deref(schema, spec);
	if (!resolved || typeof resolved !== "object") {
		return "smoke";
	}

	if (Array.isArray(resolved.oneOf) && resolved.oneOf.length > 0) {
		return sampleFromSchema(resolved.oneOf[0], spec);
	}

	if (Array.isArray(resolved.anyOf) && resolved.anyOf.length > 0) {
		return sampleFromSchema(resolved.anyOf[0], spec);
	}

	if (Array.isArray(resolved.allOf) && resolved.allOf.length > 0) {
		return sampleFromSchema(resolved.allOf[0], spec);
	}

	if (resolved.const !== undefined) {
		return resolved.const;
	}

	if (resolved.default !== undefined) {
		return resolved.default;
	}

	if (resolved.example !== undefined) {
		return resolved.example;
	}

	if (resolved.nullable || resolved.type === "null") {
		return null;
	}

	if (Array.isArray(resolved.enum) && resolved.enum.length > 0) {
		return resolved.enum[0];
	}

	if (resolved.type === "integer" || resolved.type === "number") {
		return 1;
	}

	if (resolved.type === "boolean") {
		return true;
	}

	if (resolved.format === "uuid") {
		return "00000000-0000-0000-0000-000000000000";
	}

	if (resolved.format === "date") {
		return "2024-01-01";
	}

	if (resolved.format === "date-time") {
		return "2024-01-01T00:00:00Z";
	}

	if (resolved.format === "uri") {
		return "https://example.com";
	}

	if (resolved.type === "array") {
		return [sampleFromSchema(resolved.items, spec)];
	}

	if (resolved.type === "object") {
		const properties =
			resolved.properties && typeof resolved.properties === "object"
				? Object.entries(resolved.properties)
				: [];
		if (properties.length === 0) {
			return {};
		}
		return Object.fromEntries(
			properties
				.slice(0, 4)
				.map(([key, value]) => [key, sampleFromSchema(value, spec)]),
		);
	}

	return "smoke";
}

function valueFromAuthContext(name, authContext) {
	if (!authContext) {
		return undefined;
	}

	const normalized = name.toLowerCase();
	if (normalized.includes("workspace")) {
		return authContext.workspaceId;
	}
	if (normalized.includes("session")) {
		return authContext.sessionId;
	}
	if (normalized.includes("user") || normalized.includes("principal")) {
		return authContext.userId;
	}
	return undefined;
}

function enrichSampleWithAuthContext(sample, authContext) {
	if (
		!authContext ||
		!sample ||
		typeof sample !== "object" ||
		Array.isArray(sample)
	) {
		return sample;
	}

	const patched = { ...sample };
	for (const key of Object.keys(patched)) {
		const contextual = valueFromAuthContext(key, authContext);
		if (contextual) {
			patched[key] = contextual;
		}
	}
	return patched;
}

function samplePathParam(parameter, authContext, spec) {
	const contextual = valueFromAuthContext(parameter.name, authContext);
	if (contextual) {
		return contextual;
	}

	const schema = deref(
		parameter.schema ?? parameter.content?.["application/json"]?.schema,
		spec,
	);
	const name = parameter.name.toLowerCase();

	if (schema) {
		const sample = sampleFromSchema(schema, spec);
		if (typeof sample === "string") {
			return sample;
		}
		if (sample === null || sample === undefined) {
			return "smoke";
		}
		return String(sample);
	}

	if (name.endsWith("id") || name.includes("uuid")) {
		return "00000000-0000-0000-0000-000000000000";
	}

	if (
		name.includes("page") ||
		name.includes("limit") ||
		name.includes("offset")
	) {
		return "1";
	}

	return "smoke";
}

function buildPath(pathPattern, parameters, authContext, spec) {
	const pathParams = new Map(
		parameters
			.filter((parameter) => parameter.in === "path")
			.map((parameter) => [parameter.name, parameter]),
	);

	const queryParams = parameters.filter(
		(parameter) => parameter.in === "query",
	);
	const concretePath = pathPattern.replace(
		/\{([^}]+)\}/g,
		(_match, rawName) => {
			const parameter = pathParams.get(rawName);
			const sample = parameter
				? samplePathParam(parameter, authContext, spec)
				: samplePathParam({ name: rawName }, authContext, spec);
			return encodeURIComponent(sample);
		},
	);

	const searchParams = new URLSearchParams();
	for (const parameter of queryParams) {
		const contextual = valueFromAuthContext(parameter.name, authContext);
		if (contextual) {
			searchParams.set(parameter.name, contextual);
			continue;
		}

		const schema = deref(
			parameter.schema ?? parameter.content?.["application/json"]?.schema,
			spec,
		);
		const sample = sampleFromSchema(schema, spec);
		if (sample === null || sample === undefined) {
			continue;
		}
		searchParams.set(
			parameter.name,
			typeof sample === "string" ? sample : JSON.stringify(sample),
		);
	}

	const query = searchParams.toString();
	return query ? `${concretePath}?${query}` : concretePath;
}

function pickContentType(content = {}) {
	const available = Object.keys(content);
	if (available.length === 0) {
		return undefined;
	}

	return (
		available.find((type) => JSON_CONTENT_TYPES.has(type)) ??
		available.find(
			(type) => type.startsWith("application/") && type.includes("json"),
		) ??
		available[0]
	);
}

function buildRequestInit(method, operation, spec, authContext, isProtected) {
	const headers = {
		Accept: "*/*",
	};

	if (isProtected && authContext?.token) {
		headers.Authorization = `Bearer ${authContext.token}`;
	}

	let body;
	if (MUTATING_METHODS.has(method)) {
		headers.Origin = "http://localhost:3001";
		headers["Content-Type"] = "application/json";
		body = "{}";

		const requestBody = deref(operation.requestBody, spec);
		const content = requestBody?.content ?? {};
		const contentType = pickContentType(content);
		if (contentType) {
			headers["Content-Type"] = contentType;
			const schema = deref(content[contentType]?.schema, spec);
			const sample = enrichSampleWithAuthContext(
				sampleFromSchema(schema, spec),
				authContext,
			);
			if (contentType === "application/x-www-form-urlencoded") {
				if (sample && typeof sample === "object" && !Array.isArray(sample)) {
					body = new URLSearchParams(
						Object.entries(sample).map(([key, value]) => [
							key,
							String(value ?? ""),
						]),
					).toString();
				} else {
					body = "smoke=true";
				}
			} else if (contentType === "text/plain") {
				body = typeof sample === "string" ? sample : "smoke";
			} else if (sample === null || sample === undefined) {
				body = "{}";
			} else if (typeof sample === "string") {
				body = sample;
			} else {
				body = JSON.stringify(sample);
			}
		}
	}

	return { headers, body };
}

function shouldSmokeThroughProxy(pathPattern) {
	return PROXIED_PREFIXES.some((prefix) => pathPattern.startsWith(prefix));
}

function isProtectedOperation(operation, globalSecurity) {
	if (Array.isArray(operation.security)) {
		return operation.security.length > 0;
	}
	return Array.isArray(globalSecurity) && globalSecurity.length > 0;
}

function shouldSkipOperation(pathPattern, method, operation, globalSecurity) {
	const key = operationKey(method, pathPattern);

	if (ALWAYS_SKIP_OPERATIONS.has(key)) {
		return `${key} [destructive]`;
	}

	if (
		ALWAYS_SKIP_PATH_SUBSTRINGS.some((segment) => pathPattern.includes(segment))
	) {
		return `${key} [non-smokeable]`;
	}

	const protectedOperation = isProtectedOperation(operation, globalSecurity);
	if (protectedOperation && !SEEDED_AUTH_ENABLED) {
		return `${key} [protected]`;
	}

	if (protectedOperation && MUTATING_METHODS.has(method)) {
		if (!SAFE_PROTECTED_MUTATING_OPERATIONS.has(key)) {
			return `${key} [protected-stateful]`;
		}
	}

	if (
		!protectedOperation &&
		MUTATING_METHODS.has(method) &&
		pathPattern.includes("{")
	) {
		return `${key} [stateful]`;
	}

	return null;
}

async function fetchJson(url, init = {}, allowedStatuses = [200]) {
	let response;
	try {
		response = await fetch(url, init);
	} catch (error) {
		const causeCode = error?.cause?.code ? ` (${error.cause.code})` : "";
		fail(
			`fetch ${init.method ?? "GET"} ${url} failed${causeCode}: ${error.message}`,
		);
	}

	if (response.status >= 500) {
		const responseBody = await response.text().catch(() => "");
		fail(
			`${init.method ?? "GET"} ${url} returned ${response.status}\n${responseBody || "<empty body>"}`,
		);
	}

	if (!allowedStatuses.includes(response.status)) {
		const responseBody = await response.text().catch(() => "");
		fail(
			`${init.method ?? "GET"} ${url} returned unexpected status ${response.status}\n${responseBody || "<empty body>"}`,
		);
	}

	if (response.status === 204) {
		return { response, json: null };
	}

	const payload = await response.text();
	if (!payload) {
		return { response, json: null };
	}

	try {
		return { response, json: JSON.parse(payload) };
	} catch {
		return { response, json: null };
	}
}

function leadingZeroBits(buffer) {
	let count = 0;
	for (const byte of buffer.values()) {
		if (byte === 0) {
			count += 8;
			continue;
		}
		count += Math.clz32(byte) - 24;
		break;
	}
	return count;
}

function solvePow(nonce, difficulty) {
	if (!nonce || !difficulty || difficulty <= 0) {
		return undefined;
	}

	for (let attempt = 0; attempt < 5_000_000; attempt += 1) {
		const candidate = String(attempt);
		const hash = createHash("sha256").update(`${nonce}:${candidate}`).digest();
		if (leadingZeroBits(hash) >= difficulty) {
			return candidate;
		}
	}

	fail(
		`unable to solve PoW challenge for nonce ${nonce} with difficulty ${difficulty}`,
	);
}

async function fetchPowSolution(apiBaseUrl) {
	for (let attempt = 0; attempt < 3; attempt++) {
		try {
			const response = await fetch(`${apiBaseUrl}/auth/challenge/pow`, {
				method: "GET",
				headers: {
					Accept: "*/*",
				},
			});

			if (!response.ok) {
				const text = await response.text().catch(() => "");
				console.error(
					`GET /auth/challenge/pow returned ${response.status}: ${text || "<empty>"}`,
				);
				return {};
			}

			const challenge = await response.json();
			if (!challenge || typeof challenge !== "object") {
				return {};
			}

			if (
				!challenge.nonce ||
				!challenge.difficulty ||
				challenge.difficulty <= 0
			) {
				return {};
			}

			const solution = solvePow(challenge.nonce, challenge.difficulty);
			return {
				pow_nonce: challenge.nonce,
				pow_solution: solution,
			};
		} catch (error) {
			const causeCode = error?.cause?.code ?? "";
			console.error(
				`GET /auth/challenge/pow failed (attempt ${attempt + 1}/3): ${causeCode} ${error.message}`,
			);
			await new Promise((r) => setTimeout(r, 1000 * (attempt + 1)));
		}
	}

	return {};
}

function extractSetCookies(headers) {
	if (typeof headers.getSetCookie === "function") {
		return headers.getSetCookie();
	}

	const raw = headers.get("set-cookie");
	if (!raw) {
		return [];
	}
	return raw.split(/,(?=[^;]+=)/);
}

function extractSessionTokenFromSetCookies(setCookies) {
	for (const entry of setCookies) {
		const trimmed = entry.trim();
		const match = trimmed.match(
			/^(__Host-session|session|__Host-token|token)=([^;]+)/i,
		);
		if (match) {
			return decodeURIComponent(match[2]);
		}
	}
	return undefined;
}

function seededCredentialsFromEnv() {
	const email = process.env.NVBES_SMOKE_AUTH_EMAIL;
	const password = process.env.NVBES_SMOKE_AUTH_PASSWORD;

	if ((email && !password) || (!email && password)) {
		fail(
			"NVBES_SMOKE_AUTH_EMAIL and NVBES_SMOKE_AUTH_PASSWORD must both be set when one is set",
		);
	}

	if (email && password) {
		return { email, password, shouldRegister: false };
	}

	const suffix = randomUUID();
	return {
		email: `smoke.contract.${suffix}@example.com`,
		password: `SmokeContract!${suffix.slice(0, 12)}`,
		shouldRegister: true,
	};
}

async function seedAuthContext(apiBaseUrl) {
	const credentials = seededCredentialsFromEnv();
	let workspaceName =
		process.env.NVBES_SMOKE_AUTH_WORKSPACE ?? "Smoke Contract Workspace";

	if (credentials.shouldRegister) {
		workspaceName = `Smoke Contract ${randomUUID().slice(0, 8)}`;
		const registerPow = await fetchPowSolution(apiBaseUrl);
		const registerBody = {
			email: credentials.email,
			password: credentials.password,
			firstname: "Smoke",
			lastname: "Contract",
			username: `smoke_${randomUUID().slice(0, 8)}`,
			region: "FR",
			workspace_name: workspaceName,
			...registerPow,
		};

		await fetchJson(
			`${apiBaseUrl}/auth/register`,
			{
				method: "POST",
				headers: {
					Accept: "*/*",
					"Content-Type": "application/json",
					Origin: "http://localhost:3001",
				},
				body: JSON.stringify(registerBody),
			},
			[200, 409],
		);
	}

	prepareSeededAuthAccount(credentials, workspaceName);

	const identifierPow = await fetchPowSolution(apiBaseUrl);
	const identifierBody = {
		email: credentials.email,
		...identifierPow,
	};

	const { json: identifier } = await fetchJson(
		`${apiBaseUrl}/auth/challenge/identifier`,
		{
			method: "POST",
			headers: {
				Accept: "*/*",
				"Content-Type": "application/json",
				Origin: "http://localhost:3001",
			},
			body: JSON.stringify(identifierBody),
		},
		[200],
	);

	const stateToken = identifier?.state_token;
	if (!stateToken) {
		fail(
			"seeded auth failed: /auth/challenge/identifier did not return state_token",
		);
	}

	const { response: loginResponse, json: loginBody } = await fetchJson(
		`${apiBaseUrl}/auth/challenge/pwd`,
		{
			method: "POST",
			headers: {
				Accept: "*/*",
				"Content-Type": "application/json",
				Origin: "http://localhost:3001",
			},
			body: JSON.stringify({
				state_token: stateToken,
				password: credentials.password,
			}),
		},
		[200, 202],
	);

	if (loginResponse.status === 202) {
		fail(
			"seeded auth failed: credentials require MFA, provide a non-MFA smoke account",
		);
	}

	const setCookies = extractSetCookies(loginResponse.headers);
	const token = extractSessionTokenFromSetCookies(setCookies);
	if (!token) {
		fail(
			"seeded auth failed: /auth/challenge/pwd did not set a session cookie",
		);
	}

	const authHeader = `Bearer ${token}`;
	const { json: me } = await fetchJson(`${apiBaseUrl}/auth/me`, {
		method: "GET",
		headers: {
			Accept: "*/*",
			Authorization: authHeader,
		},
	});
	const { json: sessions } = await fetchJson(`${apiBaseUrl}/auth/sessions`, {
		method: "GET",
		headers: {
			Accept: "*/*",
			Authorization: authHeader,
		},
	});

	const currentSession = Array.isArray(sessions?.sessions)
		? (sessions.sessions.find((entry) => entry.current) ?? sessions.sessions[0])
		: undefined;

	if (!me?.user?.id) {
		fail("seeded auth failed: /auth/me did not return user.id");
	}

	return {
		token,
		userId: me.user.id,
		workspaceId: me.current_workspace_id ?? undefined,
		sessionId: currentSession?.id ?? undefined,
		loginSummary: {
			email: credentials.email,
			workspaceId: me.current_workspace_id ?? null,
			sessionId: currentSession?.id ?? null,
			mfaEnabled: Boolean(loginBody?.user?.mfa_enabled),
		},
	};
}

function prepareSeededAuthAccount(credentials, workspaceName) {
	const databaseUrl = process.env.NVBES_DATABASE_URL;
	if (!databaseUrl) {
		fail("NVBES_DATABASE_URL is required for seeded-auth smoke preparation");
	}

	execFileSync(
		"cargo",
		[
			"run",
			"-q",
			"-p",
			"nvbes-account-service",
			"--",
			"--prepare-beta-e2e-account",
			"--email",
			credentials.email,
			"--password",
			credentials.password,
			"--workspace-name",
			workspaceName,
		],
		{
			cwd: ROOT_DIR,
			encoding: "utf8",
			stdio: ["ignore", "pipe", "pipe"],
			env: {
				...process.env,
				NVBES_ENV: "staging",
				NVBES_DATABASE_URL: databaseUrl,
			},
		},
	);
}

async function probe(
	baseUrl,
	path,
	method,
	operation,
	spec,
	authContext,
	protectedOperation,
) {
	const url = `${baseUrl}${path}`;
	const { headers, body } = buildRequestInit(
		method,
		operation,
		spec,
		authContext,
		protectedOperation,
	);

	await new Promise((r) => setTimeout(r, 50));

	for (let attempt = 0; attempt < 3; attempt++) {
		try {
			const response = await fetch(url, {
				method: method.toUpperCase(),
				headers,
				body,
			});

			if (response.status >= 500) {
				const responseBody = await response.text().catch(() => "");
				fail(
					`${method.toUpperCase()} ${url} returned ${response.status}\n${responseBody || "<empty body>"}`,
				);
			}

			return;
		} catch (error) {
			if (error?.cause?.code === "UND_ERR_SOCKET") {
				const wait = 1500 * (attempt + 1);
				await new Promise((r) => setTimeout(r, wait));
				continue;
			}

			const causeCode = error?.cause?.code ? ` (${error.cause.code})` : "";
			fail(
				`fetch ${method.toUpperCase()} ${url} failed${causeCode}: ${error.message}`,
			);
		}
	}

	fail(
		`fetch ${method.toUpperCase()} ${url} failed after 3 retries (UND_ERR_SOCKET)`,
	);
}

async function main() {
	const apiBaseUrl = normalizeUrl(
		process.env.NVBES_API_BASE_URL,
		"NVBES_API_BASE_URL",
	);
	const webBaseUrl = normalizeUrl(
		process.env.NVBES_WEB_BASE_URL,
		"NVBES_WEB_BASE_URL",
	);
	const spec = loadOpenApiSpec();
	const paths = spec.paths ?? {};
	const globalSecurity = spec.security;
	const authContext = SEEDED_AUTH_ENABLED
		? await seedAuthContext(apiBaseUrl)
		: undefined;

	let apiChecks = 0;
	let proxyChecks = 0;
	const skipped = [];

	for (const [pathPattern, pathItem] of Object.entries(paths)) {
		for (const [method, rawOperation] of Object.entries(pathItem)) {
			if (!ROUTABLE_METHODS.includes(method)) {
				continue;
			}

			const operation = deref(rawOperation, spec);
			const skipReason = shouldSkipOperation(
				pathPattern,
				method,
				operation,
				globalSecurity,
			);
			if (skipReason) {
				skipped.push(skipReason);
				continue;
			}

			const protectedOperation = isProtectedOperation(
				operation,
				globalSecurity,
			);
			const parameters = collectParameters(pathItem, operation, spec);
			const resolvedPath = buildPath(
				pathPattern,
				parameters,
				authContext,
				spec,
			);

			process.stderr.write(
				`#${apiChecks + 1}: ${method.toUpperCase()} ${resolvedPath}\n`,
			);
			await probe(
				apiBaseUrl,
				resolvedPath,
				method,
				operation,
				spec,
				authContext,
				protectedOperation,
			);
			apiChecks += 1;

			if (shouldSmokeThroughProxy(pathPattern) && protectedOperation) {
				await probe(
					webBaseUrl,
					resolvedPath,
					method,
					operation,
					spec,
					authContext,
					protectedOperation,
				);
				proxyChecks += 1;
			}
		}
	}

	process.stdout.write(
		`OpenAPI smoke passed: ${apiChecks} direct checks, ${proxyChecks} proxied checks\n`,
	);
	process.stdout.write(
		`Mode: ${SEEDED_AUTH_ENABLED ? "seeded-auth" : "public-only"}${SEEDED_AUTH_ENABLED ? ` (seed ${authContext.loginSummary.email})` : ""}\n`,
	);
	if (SEEDED_AUTH_ENABLED && authContext?.loginSummary) {
		process.stdout.write(
			`Seeded auth context: workspace=${authContext.loginSummary.workspaceId ?? "<none>"}, session=${authContext.loginSummary.sessionId ?? "<none>"}\n`,
		);
	}
	if (skipped.length > 0) {
		process.stdout.write(`Skipped ${skipped.length} non-smokeable checks:\n`);
		for (const entry of skipped) {
			process.stdout.write(`  - ${entry}\n`);
		}
	}
}

main().catch((error) => {
	process.stderr.write(
		`${error instanceof Error ? error.message : String(error)}\n`,
	);
	process.exit(1);
});
