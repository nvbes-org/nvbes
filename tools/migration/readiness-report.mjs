#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { readPackageScripts } from "./execution-backlog.proof.mjs";
import { serializeReadinessMarkdown } from "./readiness-report.markdown.mjs";
import { proofForReadinessArea, proofForReadinessBlocker, validateReadinessBlockerProof } from "./readiness-report.proof.mjs";
import { buildReadinessSourceRows, expectedReadinessSources } from "./readiness-report.sources.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const jsonPath = "docs/migration/readiness-report.generated.json";
const markdownPath = "docs/migration/readiness-report.md";
const errors = [];
const packageScripts = readPackageScripts(errors);

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

function readText(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return "";
	}
	return readFileSync(path, "utf8");
}

function sourceRows() {
	const rows = buildReadinessSourceRows(readJson, readText);
	const reconciliation = readJson("docs/migration/reconciliation.template.json");
	rows.push({ id: "reconciliation_template", source: "docs/migration/reconciliation.template.json", total: 1, pending: reconciliation?.decision === "go" ? 0 : 1, proof: proofForReadinessArea("reconciliation_template") });
	const liveEvidence = readJson("docs/migration/live-evidence-instances.generated.json");
	rows.push({ id: "live_evidence", source: "docs/migration/live-evidence-instances.generated.json", total: liveEvidence?.status?.requirements ?? 0, pending: liveEvidence?.status?.missing_requirements ?? 1, proof: proofForReadinessArea("live_evidence") });
	return rows;
}

function buildReport() {
	const sources = sourceRows();
	const liveEvidenceSource = sources.find((source) => source.id === "live_evidence");
	const liveEvidence = readJson("docs/migration/live-evidence-instances.generated.json");
	const blockers = sources
		.filter((source) => source.pending > 0)
		.map((source) => ({
			area: source.id,
			source: source.source,
			blocking_items: source.pending,
			proof: proofForReadinessBlocker(source.id),
		}));
	return {
		schema_version: 1,
		generation: {
			command: "node tools/migration/readiness-report.mjs --write",
			strict_cutover_command: "node tools/migration/readiness-report.mjs --strict",
		},
		status: {
			production_cutover: blockers.length === 0 ? "go" : "no-go",
			blocking_areas: blockers.length,
			blocking_items: blockers.reduce((total, blocker) => total + blocker.blocking_items, 0),
			live_evidence_missing_requirements: liveEvidenceSource?.pending ?? null,
			live_evidence_missing_items: liveEvidence?.status?.missing_evidence_items ?? null,
		},
		sources,
		blockers,
	};
}

function serializeJson(report) {
	return `${JSON.stringify(report, null, 2)}\n`;
}

function validate(report) {
	if (report.schema_version !== 1) errors.push(`${jsonPath}: schema_version must be 1`);
	if (report.generation?.command !== "node tools/migration/readiness-report.mjs --write") {
		errors.push(`${jsonPath}: generation.command is invalid`);
	}
	if (report.generation?.strict_cutover_command !== "node tools/migration/readiness-report.mjs --strict") {
		errors.push(`${jsonPath}: generation.strict_cutover_command is invalid`);
	}
	if (!["go", "no-go"].includes(report.status.production_cutover)) {
		errors.push(`${jsonPath}: unsupported production_cutover status`);
	}
	for (const field of ["blocking_areas", "blocking_items"]) {
		if (!Number.isInteger(report.status[field]) || report.status[field] < 0) errors.push(`${jsonPath}: ${field} must be a non-negative integer`);
	}
	for (const field of ["live_evidence_missing_requirements", "live_evidence_missing_items"]) {
		if (!Number.isInteger(report.status[field]) || report.status[field] < 0) errors.push(`${jsonPath}: ${field} must be a non-negative integer`);
	}
	const expected = expectedReadinessSources();
	if (!Array.isArray(report.sources) || report.sources.length !== expected.size) {
		errors.push(`${jsonPath}: sources must include all readiness inputs`);
	}
	if (!Array.isArray(report.blockers)) errors.push(`${jsonPath}: blockers must be an array`);
	validateSourceRows(report);
	validateBlockingConsistency(report);
	if (strict && report.status.production_cutover !== "go") {
		errors.push(`${jsonPath}: production cutover is ${report.status.production_cutover}`);
	}
}

function validateSourceRows(report) {
	const sources = Array.isArray(report.sources) ? report.sources : [];
	const expected = expectedReadinessSources();
	const seen = new Set();
	for (const source of sources) {
		if (!source.id) errors.push(`${jsonPath}: source id is required`);
		if (seen.has(source.id)) errors.push(`${jsonPath}: duplicate source row ${source.id}`);
		seen.add(source.id);
		if (!expected.has(source.id)) {
			errors.push(`${jsonPath}: unexpected source row ${source.id}`);
		} else if (source.source !== expected.get(source.id)) {
			errors.push(`${source.id}: source path must be ${expected.get(source.id)}`);
		}
		if (!Number.isInteger(source.total) || source.total < 0) {
			errors.push(`${source.id}: total must be a non-negative integer`);
		}
		if (source.total === 0) errors.push(`${source.id}: total must include at least one checked item`);
		if (!Number.isInteger(source.pending) || source.pending < 0) {
			errors.push(`${source.id}: pending must be a non-negative integer`);
		}
		if (Number.isInteger(source.total) && Number.isInteger(source.pending) && source.pending > source.total) {
			errors.push(`${source.id}: pending cannot exceed total`);
		}
		if (!source.proof) errors.push(`${source.id}: proof is required`);
		else errors.push(...validateReadinessBlockerProof({ area: source.id, proof: source.proof }, packageScripts));
	}
	for (const id of expected.keys()) {
		if (!seen.has(id)) errors.push(`${jsonPath}: missing source row ${id}`);
	}
}

function validateBlockingConsistency(report) {
	const sources = Array.isArray(report.sources) ? report.sources : [];
	const blockers = Array.isArray(report.blockers) ? report.blockers : [];
	const sourceBlockers = sources.filter((source) => source.pending > 0);
	const blockingItems = blockers.reduce((total, blocker) => total + blocker.blocking_items, 0);
	if (report.status.blocking_areas !== blockers.length) {
		errors.push(`${jsonPath}: blocking_areas must match blocker rows`);
	}
	if (report.status.blocking_items !== blockingItems) {
		errors.push(`${jsonPath}: blocking_items must equal blocker row total`);
	}
	if (report.status.production_cutover === "go" && blockers.length > 0) {
		errors.push(`${jsonPath}: go status cannot include blockers`);
	}
	if (report.status.production_cutover === "no-go" && blockers.length === 0) {
		errors.push(`${jsonPath}: no-go status requires at least one blocker`);
	}
	if (blockers.length !== sourceBlockers.length) {
		errors.push(`${jsonPath}: blocker rows must match sources with blocking items`);
	}
	const sourcesByArea = new Map(sources.map((source) => [source.id, source]));
	const liveEvidenceSource = sourcesByArea.get("live_evidence");
	if (report.status.live_evidence_missing_requirements !== liveEvidenceSource?.pending) {
		errors.push(`${jsonPath}: live_evidence_missing_requirements must match live_evidence source blocking count`);
	}
	for (const blocker of blockers) {
		const source = sourcesByArea.get(blocker.area);
		if (!source) {
			errors.push(`${jsonPath}: blocker ${blocker.area} has no matching source`);
			continue;
		}
		if (blocker.source !== source.source) {
			errors.push(`${jsonPath}: blocker ${blocker.area} source must match source row`);
		}
		if (blocker.blocking_items !== source.pending) {
			errors.push(`${jsonPath}: blocker ${blocker.area} count must match source blocking count`);
		}
		errors.push(...validateReadinessBlockerProof(blocker, packageScripts));
	}
}

const report = buildReport();
const json = serializeJson(report);
const markdown = serializeReadinessMarkdown(report);

if (write) {
	mkdirSync(dirname(jsonPath), { recursive: true });
	writeFileSync(jsonPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration readiness report written to ${markdownPath} and ${jsonPath}`);
	process.exit(0);
}

validate(report);

for (const [path, expected] of [[jsonPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing; run node tools/migration/readiness-report.mjs --write`);
	} else if (readFileSync(path, "utf8") !== expected) {
		errors.push(`${path}: stale; run node tools/migration/readiness-report.mjs --write`);
	}
}

if (errors.length > 0) {
	console.error("Migration readiness report checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Migration readiness report: ok (${report.status.production_cutover}, ${report.status.blocking_items} blocking items)`,
);
