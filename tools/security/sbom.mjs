#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";

const args = process.argv.slice(2);
const write = args.includes("--write");
const outputPath = "docs/migration/sbom.generated.json";
const markdownPath = "docs/migration/sbom.md";
const errors = [];

const components = [
	{ ecosystem: "node", manifest: "package.json", lockfile: "pnpm-lock.yaml" },
	{ ecosystem: "rust", manifest: "Cargo.toml", lockfile: "Cargo.lock" },
	{ ecosystem: "go", manifest: "go.mod", lockfile: "go.mod" },
	{ ecosystem: "python", manifest: "pyproject.toml", lockfile: "pyproject.toml" },
	{ ecosystem: "containers", manifest: "deploy/oss/helm/nvbes/Chart.yaml", lockfile: "deploy/oss/helm/nvbes/values.yaml" },
];

function buildSbom() {
	const entries = components.map((component) => ({
		...component,
		manifest_present: existsSync(component.manifest),
		lock_present: existsSync(component.lockfile),
	}));
	return {
		schema_version: 1,
		generation: { command: "tools/security/sbom.mjs --write" },
		summary: {
			entries: entries.length,
			complete: entries.filter((entry) => entry.manifest_present && entry.lock_present).length,
			pending: entries.filter((entry) => !entry.manifest_present || !entry.lock_present).length,
		},
		components: entries,
	};
}

function serializeJson(sbom) {
	return `${JSON.stringify(sbom, null, 2)}\n`;
}

function serializeMarkdown(sbom) {
	const lines = [
		"# SBOM Coverage Manifest",
		"",
		"## Status",
		"",
		`- entries: ${sbom.summary.entries}`,
		`- complete: ${sbom.summary.complete}`,
		`- pending: ${sbom.summary.pending}`,
		"",
		"## Components",
		"",
		"| Ecosystem | Manifest | Lockfile | Complete |",
		"|---|---|---|---:|",
	];
	for (const component of sbom.components) {
		const complete = component.manifest_present && component.lock_present;
		lines.push(`| ${component.ecosystem} | \`${component.manifest}\` | \`${component.lockfile}\` | ${complete} |`);
	}
	lines.push("", "## Regeneration", "", "```bash", "pnpm check:sbom", "tools/security/sbom.mjs --write", "```", "");
	return lines.join("\n");
}

function validate(sbom) {
	if (sbom.schema_version !== 1) errors.push(`${outputPath}: schema_version must be 1`);
	for (const component of sbom.components ?? []) {
		if (!component.manifest_present) errors.push(`${component.manifest}: missing`);
		if (!component.lock_present) errors.push(`${component.lockfile}: missing`);
	}
}

const sbom = buildSbom();
const json = serializeJson(sbom);
const markdown = serializeMarkdown(sbom);

if (write) {
	mkdirSync(dirname(outputPath), { recursive: true });
	writeFileSync(outputPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`SBOM coverage written to ${markdownPath} and ${outputPath}`);
	process.exit(0);
}

validate(sbom);
for (const [path, expected] of [[outputPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) errors.push(`${path}: missing; run tools/security/sbom.mjs --write`);
	else if (readFileSync(path, "utf8") !== expected) errors.push(`${path}: stale; run tools/security/sbom.mjs --write`);
}

if (errors.length > 0) {
	console.error("SBOM coverage checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(`SBOM coverage: ok (${sbom.summary.complete}/${sbom.summary.entries} complete)`);
