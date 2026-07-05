#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { validateInventory } from "./inventory.validation.mjs";

const outputPath = "docs/migration/inventory.generated.json";
const markdownPath = "docs/migration/inventory.md";
const args = process.argv.slice(2);
const write = args.includes("--write");

function run(command, args) {
	return execFileSync(command, args, {
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
	});
}

function files(pattern) {
	const output = run("rg", ["--files"]);
	return output
		.split("\n")
		.filter(Boolean)
		.filter((path) => pattern.test(path))
		.filter((path) => !path.includes("/target/") && !path.includes("/node_modules/"))
		.sort();
}

function unique(values) {
	return [...new Set(values)].sort();
}

function read(path) {
	return readFileSync(path, "utf8");
}

function routeInventory() {
	return files(/^apps\/.*\/src\/.*routes.*\.(rs|tsx?|jsx?)$/)
		.map((file) => {
			const content = read(file);
			const endpoints = unique([
				...content.matchAll(/path\s*=\s*"([^"]+)"/g),
				...content.matchAll(/\.route\(\s*"([^"]+)"/g),
			].map((match) => match[1]).filter((path) => path.startsWith("/")));
			return { file, endpoints };
		})
		.filter((entry) => entry.endpoints.length > 0);
}

function tableInventory() {
	return files(/^apps\/.*\/migrations\/.*\.sql$/)
		.map((file) => {
			const content = read(file);
			const tables = unique(
				[...content.matchAll(/CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?([a-zA-Z0-9_".]+)/gi)].map(
					(match) => match[1].replaceAll('"', ""),
				),
			);
			return { file, tables };
		})
		.filter((entry) => entry.tables.length > 0);
}

function jobInventory() {
	return files(/^apps\/.*\/src\/.*(worker|job).*\.rs$|^libs\/rust\/.*\/src\/.*(worker|job).*\.rs$/)
		.map((file) => {
			const content = read(file);
			const constants = unique(
				[...content.matchAll(/(?:pub\s+)?const\s+(JOB_[A-Z0-9_]+)\s*:[^=]+=\s*"([^"]+)"/g)].map(
					(match) => `${match[1]}=${match[2]}`,
				),
			);
			const jobTypes = unique(
				[...content.matchAll(/job_type\s*:\s*(JOB_[A-Z0-9_]+)/g)].map((match) => match[1]),
			);
			return { file, constants, jobTypes };
		})
		.filter((entry) => entry.constants.length > 0 || entry.jobTypes.length > 0);
}

function infrastructureInventory() {
	return files(
		/^(deploy|infrastructure)\/.*(\.tf|\.ya?ml|\.json|Dockerfile|docker-compose.*|README\.md)$/,
	);
}

function documentInventory() {
	return files(/^docs\/.*\.md$/);
}

function secretInventory() {
	const envFiles = files(/(^|\/)\.env(\.example)?$/);
	return envFiles.map((file) => {
		const keys = unique(
			read(file)
				.split("\n")
				.map((line) => line.trim())
				.filter((line) => line && !line.startsWith("#"))
				.map((line) => line.match(/^([A-Z][A-Z0-9_]*)=/)?.[1])
				.filter(Boolean),
		);
		return { file, keys };
	});
}

function resourceInventory() {
	const resources = [];
	for (const fileEntry of jobInventory()) {
		for (const constant of fileEntry.constants) {
			const [symbol, name] = constant.split("=");
			resources.push({ type: "queue", source: fileEntry.file, name, symbol });
		}
	}
	for (const file of files(/^infrastructure\/.*\.tf$/)) {
		const content = read(file);
		for (const match of content.matchAll(/resource\s+"scaleway_object_bucket"\s+"([^"]+)"/g)) {
			resources.push({ type: "bucket", source: file, name: `scaleway_object_bucket.${match[1]}` });
		}
	}
	if (existsSync("contracts/events/manifest.json")) {
		const manifest = JSON.parse(read("contracts/events/manifest.json"));
		for (const event of manifest.events ?? []) {
			resources.push({
				type: "event_topic",
				source: "contracts/events/manifest.json",
				name: event.event_type,
				schema: event.schema,
				critical: event.critical === true,
			});
		}
	}
	return resources.sort((a, b) => `${a.type}:${a.name}:${a.source}`.localeCompare(`${b.type}:${b.name}:${b.source}`));
}

const inventory = {
	schema_version: 1,
	generation: {
		command: "tools/migration/inventory.mjs --write",
		deterministic: true,
	},
	summary: {},
	routes: routeInventory(),
	tables: tableInventory(),
	jobs: jobInventory(),
	infrastructure: infrastructureInventory(),
	docs: documentInventory(),
	secrets: secretInventory(),
	resources: resourceInventory(),
};

inventory.summary = {
	route_files: inventory.routes.length,
	endpoints: inventory.routes.reduce((sum, entry) => sum + entry.endpoints.length, 0),
	migration_files: inventory.tables.length,
	tables: inventory.tables.reduce((sum, entry) => sum + entry.tables.length, 0),
	job_files: inventory.jobs.length,
	job_references: inventory.jobs.reduce(
		(sum, entry) => sum + entry.constants.length + entry.jobTypes.length,
		0,
	),
	infrastructure_files: inventory.infrastructure.length,
	doc_files: inventory.docs.length,
	secret_files: inventory.secrets.length,
	secret_keys: inventory.secrets.reduce((sum, entry) => sum + entry.keys.length, 0),
	resources: inventory.resources.length,
};

function inline(value) {
	return JSON.stringify(value);
}

function serializeEntries(entries, key) {
	return entries.map((entry) => `    ${inline(entry)}`).join(",\n") || "";
}

function serializeArray(values) {
	return values.map((value) => `    ${inline(value)}`).join(",\n") || "";
}

function serializeInventory(value) {
	return `{
  "schema_version": ${value.schema_version},
  "generation": ${inline(value.generation)},
  "summary": ${inline(value.summary)},
  "routes": [
${serializeEntries(value.routes)}
  ],
  "tables": [
${serializeEntries(value.tables)}
  ],
  "jobs": [
${serializeEntries(value.jobs)}
  ],
  "infrastructure": [
${serializeArray(value.infrastructure)}
  ],
  "docs": [
${serializeArray(value.docs)}
  ],
  "secrets": [
${serializeEntries(value.secrets)}
  ],
  "resources": [
${serializeEntries(value.resources)}
  ]
}
`;
}

const serialized = serializeInventory(inventory);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, serialized);
	console.log(`Migration inventory written to ${outputPath}`);
	process.exit(0);
}

if (!existsSync(outputPath)) {
	console.error(`${outputPath}: missing; run tools/migration/inventory.mjs --write`);
	process.exit(1);
}

const validationErrors = validateInventory(inventory, outputPath);
validationErrors.push(...validateMarkdownInventory());
if (validationErrors.length > 0) {
	console.error("Migration inventory checks failed:");
	for (const error of validationErrors) console.error(`- ${error}`);
	process.exit(1);
}

const current = readFileSync(outputPath, "utf8");
if (current !== serialized) {
	console.error(`${outputPath}: stale; run tools/migration/inventory.mjs --write`);
	process.exit(1);
}

console.log(`Migration inventory: ok (${inventory.summary.endpoints} endpoints, ${inventory.summary.tables} tables)`);

function validateMarkdownInventory() {
	const errors = [];
	const markdown = readFileSync(markdownPath, "utf8");
	const domains = markdownRowsAfter(markdown, "## Domains");
	const sourceElements = markdownRowsAfter(markdown, "## Source Elements");
	const expected = {
		total_domain_rows: domains.length,
		pending_domain_rows: domains.filter((row) => row[2] === "pending inventory").length,
		total_source_element_rows: sourceElements.length,
		pending_source_element_rows: sourceElements.filter((row) => row[2] === "pending").length,
	};
	for (const [key, value] of Object.entries(expected)) {
		const actual = markdownStatusNumber(markdown, key);
		if (actual === null) errors.push(`${markdownPath}: missing status summary ${key}`);
		else if (actual !== value) errors.push(`${markdownPath}: ${key} expected ${value}, found ${actual}`);
	}
	return errors;
}

function markdownRowsAfter(markdown, heading) {
	const start = markdown.indexOf(heading);
	if (start === -1) return [];
	const section = markdown.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---") && !line.includes("Domain |") && !line.includes("Element |"))
		.map((line) => line.split("|").slice(1, -1).map((cell) => cell.trim()));
}

function markdownStatusNumber(markdown, key) {
	const match = markdown.match(new RegExp(`- \`${key}\`: (\\d+)`));
	return match ? Number.parseInt(match[1], 10) : null;
}
