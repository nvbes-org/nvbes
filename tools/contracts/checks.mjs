#!/usr/bin/env node
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const errors = [];

function readJson(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return undefined;
	}

	try {
		return JSON.parse(readFileSync(path, "utf8"));
	} catch (error) {
		errors.push(`${path}: invalid JSON: ${error.message}`);
		return undefined;
	}
}

function walk(dir, predicate, results = []) {
	if (!existsSync(dir)) return results;
	for (const entry of readdirSync(dir)) {
		const path = join(dir, entry);
		const stat = statSync(path);
		if (stat.isDirectory()) {
			walk(path, predicate, results);
		} else if (predicate(path)) {
			results.push(path);
		}
	}
	return results;
}

function routePathsFromSource(dir, api) {
	const files = walk(dir, (path) => path.endsWith(".rs"));
	const paths = new Map();

	for (const file of files) {
		const content = readFileSync(file, "utf8");
		for (const match of content.matchAll(/path\s*=\s*"([^"]+)"/g)) {
			const path = match[1];
			if (!path.startsWith("/")) continue;
			if (api.versionPrefix && !path.startsWith(api.versionPrefix)) continue;
			if (!paths.has(path)) paths.set(path, []);
			paths.get(path).push(file);
		}
	}

	return paths;
}

function checkOpenApi() {
	const manifest = readJson("contracts/openapi/manifest.json");
	if (!manifest) return;
	if (!Array.isArray(manifest.apis) || manifest.apis.length === 0) {
		errors.push("contracts/openapi/manifest.json: apis must be a non-empty array");
		return;
	}

	for (const api of manifest.apis) {
		if (!api.name || !api.document || !api.surface) {
			errors.push("contracts/openapi/manifest.json: every api needs name, surface and document");
			continue;
		}

		const spec = readJson(api.document);
		if (!spec) continue;
		if (typeof spec.openapi !== "string" || !spec.openapi.startsWith("3.")) {
			errors.push(`${api.document}: OpenAPI 3.x document expected`);
		}
		if (!spec.info?.title || !spec.info?.version) {
			errors.push(`${api.document}: info.title and info.version are required`);
		}
		const paths = Object.keys(spec.paths ?? {});
		if (paths.length === 0) {
			errors.push(`${api.document}: at least one path is required`);
		}
		if (api.surface === "public" && api.versionPrefix) {
			if (!paths.some((path) => path.startsWith(api.versionPrefix))) {
				errors.push(`${api.document}: public API must expose ${api.versionPrefix} paths`);
			}
		}
		if (api.routeSource) {
			const sourcePaths = routePathsFromSource(api.routeSource, api);
			const specPaths = new Set(paths);
			for (const [path, files] of sourcePaths.entries()) {
				if (!specPaths.has(path)) {
					errors.push(
						`${api.document}: route ${path} from ${files[0]} is missing from OpenAPI`,
					);
				}
			}
		}
	}
}

function checkProto() {
	const files = walk("contracts/protobuf", (path) => path.endsWith(".proto"));
	if (files.length === 0) {
		errors.push("contracts/protobuf: at least one .proto file is required");
	}

	for (const file of files) {
		const content = readFileSync(file, "utf8");
		if (!content.includes('syntax = "proto3";')) {
			errors.push(`${file}: proto3 syntax is required`);
		}
		if (!/^package\s+nvbes\.[a-z0-9_.]+\.v\d+;/m.test(content)) {
			errors.push(`${file}: package must be versioned under nvbes.*.vN`);
		}
		if (/\bTODO\b|\bTBD\b|\bFIXME\b/i.test(content)) {
			errors.push(`${file}: unresolved marker`);
		}
	}
}

function checkEvents() {
	const manifest = readJson("contracts/events/manifest.json");
	const envelope = readJson("contracts/events/envelope.schema.json");
	if (!manifest || !envelope) return;

	const requiredEnvelopeFields = [
		"event_id",
		"event_type",
		"event_version",
		"tenant_id",
		"region_id",
		"occurred_at",
		"correlation_id",
		"idempotency_key",
		"payload",
	];
	for (const field of requiredEnvelopeFields) {
		if (!envelope.required?.includes(field)) {
			errors.push(`contracts/events/envelope.schema.json: missing required field ${field}`);
		}
	}

	if (!Array.isArray(manifest.events) || manifest.events.length === 0) {
		errors.push("contracts/events/manifest.json: events must be a non-empty array");
		return;
	}

	const seen = new Set();
	for (const event of manifest.events) {
		const key = `${event.event_type}@${event.event_version}`;
		if (seen.has(key)) {
			errors.push(`contracts/events/manifest.json: duplicate event ${key}`);
		}
		seen.add(key);

		if (!event.critical) {
			errors.push(`contracts/events/manifest.json: ${key} must declare critical=true or move out of this manifest`);
		}

		const schema = readJson(event.schema);
		if (!schema) continue;
		if (schema.properties?.event_type?.const !== event.event_type) {
			errors.push(`${event.schema}: event_type const must match manifest`);
		}
		if (schema.properties?.event_version?.const !== event.event_version) {
			errors.push(`${event.schema}: event_version const must match manifest`);
		}
		if (!schema.properties?.payload || !schema.required?.includes("payload")) {
			errors.push(`${event.schema}: payload property is required`);
		}
	}
}

checkOpenApi();
checkProto();
checkEvents();

if (errors.length > 0) {
	console.error("Contract checks failed:");
	for (const error of errors) {
		console.error(`- ${error}`);
	}
	process.exit(1);
}

console.log("Contracts: ok");
