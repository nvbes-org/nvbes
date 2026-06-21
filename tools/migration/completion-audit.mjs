#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import { serializeCompletionMarkdown } from "./completion-audit.markdown.mjs";
import { buildCompletionRequirements } from "./completion-requirements.mjs";
import { readPackageScripts, validateProofCommand } from "./execution-backlog.proof.mjs";

const args = process.argv.slice(2);
const write = args.includes("--write");
const strict = args.includes("--strict");
const jsonPath = "docs/migration/completion-audit.generated.json";
const markdownPath = "docs/migration/completion-audit.md";
const readinessPath = "docs/migration/readiness-report.generated.json";
const gateEvidencePath = "docs/migration/gate-evidence.generated.json";
const targetStructurePath = "docs/migration/target-structure.generated.json";
const codegenPath = "docs/migration/codegen.generated.json";
const supplyChainPath = "docs/migration/supply-chain.generated.json";
const runtimeFoundationPath = "docs/migration/runtime-foundation.generated.json";
const phaseLedgerPath = "docs/migration/phase-ledger.generated.json";
const domainLedgerPath = "docs/migration/domain-ledger.generated.json";
const domainDodPath = "docs/migration/domain-dod.generated.json";
const liveEvidencePath = "docs/migration/live-evidence-instances.generated.json";
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

function read(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return "";
	}
	return readFileSync(path, "utf8");
}

function markerCount(path) {
	const content = read(path).toLowerCase();
	const matches = content.match(/\b(no-go|pending|blocking|not run|not scheduled|none)\b/g);
	return matches?.length ?? 0;
}

function gateSummary(gateEvidence) {
	const gates = gateEvidence?.gates ?? [];
	return {
		total: gates.length,
		go: gates.filter((gate) => gate.decision === "go").length,
		withEvidence: gates.filter((gate) => Array.isArray(gate.evidence) && gate.evidence.length > 0).length,
	};
}

function ledgerSummary(ledger, key) {
	const rows = ledger?.[key] ?? [];
	return {
		total: rows.length,
		go: rows.filter((row) => row.decision === "go").length,
		withEvidence: rows.filter((row) => {
			if (Array.isArray(row.evidence)) return row.evidence.length > 0;
			return Array.isArray(row.implementation_evidence) && row.implementation_evidence.length > 0
				&& Array.isArray(row.migration_evidence) && row.migration_evidence.length > 0;
		}).length,
		accepted: rows.filter((row) => row.status === "passed" || row.status === "accepted").length,
	};
}

function buildAudit() {
	const readiness = readJson(readinessPath);
	const gateEvidence = readJson(gateEvidencePath);
	const targetStructure = readJson(targetStructurePath);
	const codegen = readJson(codegenPath);
	const supplyChain = readJson(supplyChainPath);
	const runtimeFoundation = readJson(runtimeFoundationPath);
	const phaseLedger = readJson(phaseLedgerPath);
	const domainLedger = readJson(domainLedgerPath);
	const domainDodLedger = readJson(domainDodPath);
	const liveEvidence = readJson(liveEvidencePath);
	const gates = gateSummary(gateEvidence);
	const runtimes = ledgerSummary(runtimeFoundation, "runtimes");
	const phases = ledgerSummary(phaseLedger, "phases");
	const domains = ledgerSummary(domainLedger, "domains");
	const domainDod = ledgerSummary(domainDodLedger, "entries");
	const readinessGo = readiness?.status?.production_cutover === "go";
	const noReadinessBlockers = (readiness?.status?.blocking_items ?? 1) === 0;
	const targetStructureReady = (targetStructure?.summary?.entries ?? 0) > 0 && (targetStructure?.summary?.pending ?? 1) === 0;
	const codegenReady = (codegen?.summary?.entries ?? 0) > 0 && (codegen?.summary?.pending ?? 1) === 0;
	const supplyChainReady = (supplyChain?.summary?.entries ?? 0) > 0 && (supplyChain?.summary?.pending ?? 1) === 0;
	const allGatesGo = gates.total > 0 && gates.go === gates.total && gates.withEvidence === gates.total;
	const allRuntimesGo = runtimes.total > 0 && runtimes.go === runtimes.total && runtimes.withEvidence === runtimes.total && runtimes.accepted === runtimes.total;
	const allPhasesGo = phases.total > 0 && phases.go === phases.total && phases.withEvidence === phases.total && phases.accepted === phases.total;
	const allDomainsGo = domains.total > 0 && domains.go === domains.total && domains.withEvidence === domains.total && domains.accepted === domains.total;
	const allDomainDodGo = domainDod.total > 0 && domainDod.go === domainDod.total && domainDod.withEvidence === domainDod.total && domainDod.accepted === domainDod.total;
	const liveEvidenceReady = liveEvidence?.status?.decision === "go" && (liveEvidence?.status?.missing_requirements ?? 1) === 0;
	const strictReady = readinessGo && noReadinessBlockers && liveEvidenceReady && allGatesGo && allRuntimesGo && allPhasesGo && allDomainsGo && allDomainDodGo;
	const requirements = buildCompletionRequirements({
		allDomainsGo,
		allDomainDodGo,
		allGatesGo,
		allRuntimesGo,
		allPhasesGo,
		gateEvidencePath,
		liveEvidencePath,
		liveEvidenceReady,
		markerCount,
		noReadinessBlockers,
		readinessGo,
		readinessPath,
		scripts: packageScripts,
		codegenReady,
		strictReady,
		supplyChainReady,
		targetStructureReady,
	});

	const incomplete = requirements.filter((entry) => entry.status !== "complete" && entry.status !== "control-active");
	return {
		schema_version: 1,
		generation: {
			command: "tools/migration/completion-audit.mjs --write",
			strict_completion_command: "tools/migration/completion-audit.mjs --strict",
		},
		status: {
			objective: incomplete.length === 0 ? "complete" : "incomplete",
			requirements: requirements.length,
			incomplete: incomplete.length,
			readiness: readiness?.status?.production_cutover ?? "unknown",
			readiness_blocking_items: readiness?.status?.blocking_items ?? null,
			runtimes_go: runtimes.go,
			runtimes_total: runtimes.total,
			domains_go: domains.go,
			domains_total: domains.total,
			domain_dod_go: domainDod.go,
			domain_dod_total: domainDod.total,
			gates_go: gates.go,
			gates_total: gates.total,
			phases_go: phases.go,
			phases_total: phases.total,
			live_evidence_decision: liveEvidence?.status?.decision ?? "unknown",
			live_evidence_missing_requirements: liveEvidence?.status?.missing_requirements ?? null,
			live_evidence_missing_items: liveEvidence?.status?.missing_evidence_items ?? null,
		},
		requirements,
	};
}

function serializeJson(audit) {
	return `${JSON.stringify(audit, null, 2)}\n`;
}

function validate(audit) {
	if (audit.schema_version !== 1) errors.push(`${jsonPath}: schema_version must be 1`);
	validateGeneration(audit);
	if (!["complete", "incomplete"].includes(audit.status.objective)) {
		errors.push(`${jsonPath}: unsupported objective status`);
	}
	if (!Array.isArray(audit.requirements) || audit.requirements.length < 10) {
		errors.push(`${jsonPath}: requirements must include blueprint and runbook completion items`);
	}
	validateStatusShape(audit);
	validateStatusConsistency(audit);
	for (const item of audit.requirements ?? []) {
		if (!item.id) errors.push(`${jsonPath}: requirement id is required`);
		if (!isConcreteText(item.source)) errors.push(`${item.id}: source is required`);
		if (!isConcreteText(item.requirement)) errors.push(`${item.id}: requirement text is required`);
		if (!["complete", "blocked", "control-active", "missing"].includes(item.status)) {
			errors.push(`${item.id}: unsupported status ${item.status}`);
		}
		if (!Array.isArray(item.evidence) || item.evidence.length === 0) {
			errors.push(`${item.id}: evidence is required`);
		} else {
			for (const evidence of item.evidence) {
				if (!isConcreteEvidencePath(evidence)) continue;
				if (!existsSync(evidence)) errors.push(`${item.id}: evidence path is missing: ${evidence}`);
			}
		}
		if (!item.proof) errors.push(`${item.id}: proof command is required`);
		else errors.push(...validateProofCommand(item, packageScripts));
	}
	if (strict && audit.status.objective !== "complete") {
		errors.push(`${jsonPath}: objective is ${audit.status.objective}`);
	}
}

function validateGeneration(audit) {
	const generation = audit.generation ?? {};
	if (generation.command !== "tools/migration/completion-audit.mjs --write") errors.push(`${jsonPath}: generation.command is invalid`);
	if (generation.strict_completion_command !== "tools/migration/completion-audit.mjs --strict") errors.push(`${jsonPath}: generation.strict_completion_command is invalid`);
}

function validateStatusShape(audit) {
	const status = audit.status ?? {};
	if (!["complete", "incomplete"].includes(status.objective)) errors.push(`${jsonPath}: status.objective is invalid`);
	if (!["go", "no-go", "unknown"].includes(status.readiness)) errors.push(`${jsonPath}: status.readiness is invalid`);
	if (!["go", "no-go", "unknown"].includes(status.live_evidence_decision)) errors.push(`${jsonPath}: status.live_evidence_decision is invalid`);
	for (const field of ["requirements", "incomplete", "readiness_blocking_items", "live_evidence_missing_requirements", "live_evidence_missing_items"]) if (!Number.isInteger(status[field]) || status[field] < 0) errors.push(`${jsonPath}: status.${field} must be a non-negative integer`);
	for (const prefix of ["runtimes", "domains", "domain_dod", "gates", "phases"]) {
		const go = status[`${prefix}_go`], total = status[`${prefix}_total`];
		if (!Number.isInteger(go) || go < 0 || !Number.isInteger(total) || total < 0 || go > total) errors.push(`${jsonPath}: status ${prefix}_go/${prefix}_total is invalid`);
	}
}

function validateStatusConsistency(audit) {
	const requirements = Array.isArray(audit.requirements) ? audit.requirements : [];
	const incomplete = requirements.filter((entry) => entry.status !== "complete" && entry.status !== "control-active");
	if (audit.status.requirements !== requirements.length) errors.push(`${jsonPath}: status requirements must match requirement rows`);
	if (audit.status.incomplete !== incomplete.length) errors.push(`${jsonPath}: status incomplete must match incomplete requirement rows`);
	if (audit.status.objective === "complete") validateCompleteObjective(audit, incomplete);
	if (audit.status.objective === "incomplete" && incomplete.length === 0) errors.push(`${jsonPath}: incomplete objective requires at least one incomplete row`);
}

function validateCompleteObjective(audit, incomplete) {
	if (incomplete.length > 0) errors.push(`${jsonPath}: complete objective requires zero incomplete rows`);
	if (audit.status.readiness !== "go" || audit.status.readiness_blocking_items !== 0) errors.push(`${jsonPath}: complete objective requires readiness go with zero blockers`);
	if (audit.status.live_evidence_decision !== "go" || audit.status.live_evidence_missing_requirements !== 0 || audit.status.live_evidence_missing_items !== 0) errors.push(`${jsonPath}: complete objective requires live evidence go with zero missing requirements and evidence items`);
	for (const prefix of ["runtimes", "domains", "domain_dod", "gates", "phases"]) if (audit.status[`${prefix}_total`] < 1 || audit.status[`${prefix}_go`] !== audit.status[`${prefix}_total`]) errors.push(`${jsonPath}: complete objective requires ${prefix}_go to equal ${prefix}_total`);
}

function isConcreteEvidencePath(evidence) {
	return typeof evidence === "string" && !/[<>*]/.test(evidence);
}

function isConcreteText(value) {
	return typeof value === "string" && value.trim().length > 0 && !/\b(tbd|todo|placeholder|unknown)\b/i.test(value);
}

const audit = buildAudit();
const json = serializeJson(audit);
const markdown = serializeCompletionMarkdown(audit);

if (write) {
	mkdirSync(dirname(jsonPath), { recursive: true });
	writeFileSync(jsonPath, json);
	writeFileSync(markdownPath, markdown);
	console.log(`Migration completion audit written to ${markdownPath} and ${jsonPath}`);
	process.exit(0);
}

validate(audit);

for (const [path, expected] of [[jsonPath, json], [markdownPath, markdown]]) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing; run tools/migration/completion-audit.mjs --write`);
	} else if (readFileSync(path, "utf8") !== expected) {
		errors.push(`${path}: stale; run tools/migration/completion-audit.mjs --write`);
	}
}

if (errors.length > 0) {
	console.error("Migration completion audit checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Migration completion audit: ok (${audit.status.objective}, ${audit.status.incomplete} incomplete requirements)`,
);
