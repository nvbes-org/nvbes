#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";
import { validatePacketLiveEvidenceAlignment } from "./status-consistency.live-evidence.mjs";

const paths = {
	readiness: "docs/migration/readiness-report.generated.json",
	packet: "docs/migration/cutover-evidence-packet.generated.json",
	liveEvidence: "docs/migration/live-evidence-instances.generated.json",
	completion: "docs/migration/completion-audit.generated.json",
	backlog: "docs/migration/execution-backlog.generated.json",
	runtimeFoundation: "docs/migration/runtime-foundation.generated.json",
	domainLedger: "docs/migration/domain-ledger.generated.json",
	domainDod: "docs/migration/domain-dod.generated.json",
	gateEvidence: "docs/migration/gate-evidence.generated.json",
	phaseLedger: "docs/migration/phase-ledger.generated.json",
	riskRegister: "docs/migration/risk-register.generated.json",
};
const errors = [];

function readJson(label, path) {
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

const readiness = readJson("readiness", paths.readiness);
const packet = readJson("packet", paths.packet);
const liveEvidence = readJson("liveEvidence", paths.liveEvidence);
const completion = readJson("completion", paths.completion);
const backlog = readJson("backlog", paths.backlog);
const runtimeFoundation = readJson("runtimeFoundation", paths.runtimeFoundation);
const domainLedger = readJson("domainLedger", paths.domainLedger);
const domainDod = readJson("domainDod", paths.domainDod);
const gateEvidence = readJson("gateEvidence", paths.gateEvidence);
const phaseLedger = readJson("phaseLedger", paths.phaseLedger);
const riskRegister = readJson("riskRegister", paths.riskRegister);

const readinessStatus = readiness?.status?.production_cutover;
const packetStatus = packet?.status;
const liveStatus = liveEvidence?.status;
const completionStatus = completion?.status;
const backlogStatus = backlog?.status;
const blockers = readiness?.status?.blocking_items ?? null;
const missingLiveEvidence = liveStatus?.missing_requirements ?? null;
const missingLiveEvidenceItems = liveStatus?.missing_evidence_items ?? null;

validateReadinessCounters(readiness);
validateCompletionCounters(completion);
validateCompletionSourceCounters(completion, {
	runtimeFoundation,
	domainLedger,
	domainDod,
	gateEvidence,
	phaseLedger,
});
validateCompletionRiskClosure(completion, riskRegister);
validateBacklogCounters(backlog);
validateLiveEvidenceCounters(liveEvidence);
validatePacketLiveEvidenceAlignment(packet, liveEvidence, errors);
validateReadinessLiveEvidence(readiness, missingLiveEvidence);

if (packetStatus?.readiness !== readinessStatus) {
	errors.push(`cutover packet readiness ${packetStatus?.readiness} must match readiness report ${readinessStatus}`);
}
if (completionStatus?.readiness !== readinessStatus) {
	errors.push(`completion readiness ${completionStatus?.readiness} must match readiness report ${readinessStatus}`);
}
if (backlogStatus?.readiness !== readinessStatus) {
	errors.push(`backlog readiness ${backlogStatus?.readiness} must match readiness report ${readinessStatus}`);
}
if (completionStatus?.readiness_blocking_items !== blockers) {
	errors.push(`completion readiness_blocking_items ${completionStatus?.readiness_blocking_items} must match ${blockers}`);
}
if (completionStatus?.live_evidence_decision !== liveStatus?.decision) {
	errors.push(`completion live evidence decision ${completionStatus?.live_evidence_decision} must match ${liveStatus?.decision}`);
}
if (backlogStatus?.live_evidence !== liveStatus?.decision) {
	errors.push(`backlog live evidence decision ${backlogStatus?.live_evidence} must match ${liveStatus?.decision}`);
}
if (completionStatus?.live_evidence_missing_requirements !== missingLiveEvidence) {
	errors.push(`completion missing live evidence ${completionStatus?.live_evidence_missing_requirements} must match ${missingLiveEvidence}`);
}
if (completionStatus?.live_evidence_missing_items !== missingLiveEvidenceItems) {
	errors.push(`completion missing live evidence items ${completionStatus?.live_evidence_missing_items} must match ${missingLiveEvidenceItems}`);
}
if (readiness?.status?.live_evidence_missing_requirements !== missingLiveEvidence) {
	errors.push(`readiness missing live evidence ${readiness?.status?.live_evidence_missing_requirements} must match ${missingLiveEvidence}`);
}
if (readiness?.status?.live_evidence_missing_items !== missingLiveEvidenceItems) {
	errors.push(`readiness missing live evidence items ${readiness?.status?.live_evidence_missing_items} must match ${missingLiveEvidenceItems}`);
}
if (backlogStatus?.live_evidence_missing_requirements !== missingLiveEvidence) {
	errors.push(`backlog missing live evidence ${backlogStatus?.live_evidence_missing_requirements} must match ${missingLiveEvidence}`);
}
if (backlogStatus?.live_evidence_missing_items !== missingLiveEvidenceItems) {
	errors.push(`backlog missing live evidence items ${backlogStatus?.live_evidence_missing_items} must match ${missingLiveEvidenceItems}`);
}
if ((packetStatus?.live_blocking_items ?? null) !== missingLiveEvidence) {
	errors.push(`cutover packet live blockers ${packetStatus?.live_blocking_items} must match missing live evidence ${missingLiveEvidence}`);
}
if (packetStatus?.completion !== (completionStatus?.objective === "complete" ? "go" : "no-go")) {
	errors.push(`cutover packet completion ${packetStatus?.completion} must match completion objective ${completionStatus?.objective}`);
}
if (packetStatus?.backlog_open_tasks !== backlogStatus?.open_tasks) {
	errors.push(`cutover packet backlog_open_tasks ${packetStatus?.backlog_open_tasks} must match backlog open tasks ${backlogStatus?.open_tasks}`);
}
if (packetStatus?.backlog_blocking_items !== backlogStatus?.blocking_items) {
	errors.push(`cutover packet backlog_blocking_items ${packetStatus?.backlog_blocking_items} must match backlog blocking items ${backlogStatus?.blocking_items}`);
}
if (readinessStatus === "go" && blockers !== 0) {
	errors.push(`readiness go requires zero blocking items, got ${blockers}`);
}
if (readinessStatus === "go" && liveStatus?.decision !== "go") {
	errors.push(`readiness go requires live evidence go, got ${liveStatus?.decision}`);
}
if (readinessStatus === "no-go" && blockers === 0) {
	errors.push("readiness no-go requires at least one blocking item");
}
if (liveStatus?.decision === "go" && missingLiveEvidence !== 0) {
	errors.push(`live evidence go requires zero missing requirements, got ${missingLiveEvidence}`);
}
if (liveStatus?.decision === "go" && missingLiveEvidenceItems !== 0) {
	errors.push(`live evidence go requires zero missing evidence items, got ${missingLiveEvidenceItems}`);
}
if (packetStatus?.decision === "go" && (readinessStatus !== "go" || liveStatus?.decision !== "go")) {
	errors.push("cutover packet go requires readiness go and live evidence go");
}
if (packetStatus?.decision === "go" && completionStatus?.objective !== "complete") {
	errors.push(`cutover packet go requires completion complete, got ${completionStatus?.objective}`);
}
if (packetStatus?.decision === "go" && (backlogStatus?.open_tasks !== 0 || backlogStatus?.blocking_items !== 0)) {
	errors.push(`cutover packet go requires zero backlog tasks and blockers, got ${backlogStatus?.open_tasks}/${backlogStatus?.blocking_items}`);
}
if (completionStatus?.objective === "complete" && (completionStatus?.incomplete ?? 1) !== 0) {
	errors.push(`completion complete requires zero incomplete requirements, got ${completionStatus?.incomplete}`);
}
if (completionStatus?.objective === "complete" && (readinessStatus !== "go" || liveStatus?.decision !== "go")) {
	errors.push("completion complete requires readiness go and live evidence go");
}
if (backlogStatus?.completion !== completionStatus?.objective) {
	errors.push(`backlog completion ${backlogStatus?.completion} must match completion objective ${completionStatus?.objective}`);
}
if (backlogStatus?.completion === "complete" && (backlogStatus?.open_tasks ?? 1) !== 0) {
	errors.push(`complete backlog requires zero open tasks, got ${backlogStatus?.open_tasks}`);
}
if ((backlogStatus?.open_tasks ?? 1) === 0 && completionStatus?.objective !== "complete") {
	errors.push(`zero-open-task backlog requires completion complete, got ${completionStatus?.objective}`);
}

function validateReadinessCounters(report) {
	const sources = Array.isArray(report?.sources) ? report.sources : [];
	const blockers = Array.isArray(report?.blockers) ? report.blockers : [];
	const blockingItems = blockers.reduce((total, blocker) => total + blocker.blocking_items, 0);
	if (report?.status?.blocking_areas !== blockers.length) {
		errors.push(`readiness blocking_areas ${report?.status?.blocking_areas} must match blocker rows ${blockers.length}`);
	}
	if (report?.status?.blocking_items !== blockingItems) {
		errors.push(`readiness blocking_items ${report?.status?.blocking_items} must match blocker total ${blockingItems}`);
	}
	const sourceByArea = new Map(sources.map((source) => [source.id, source]));
	for (const blocker of blockers) {
		const source = sourceByArea.get(blocker.area);
		if (!source) {
			errors.push(`readiness blocker ${blocker.area} must match a source row`);
		} else if (source.pending !== blocker.blocking_items) {
			errors.push(`readiness blocker ${blocker.area} must match source pending count ${source.pending}`);
		}
	}
}

function validateCompletionCounters(report) {
	const requirements = Array.isArray(report?.requirements) ? report.requirements : [];
	const incomplete = requirements.filter((entry) => entry.status !== "complete" && entry.status !== "control-active");
	if (report?.status?.requirements !== requirements.length) {
		errors.push(`completion requirements ${report?.status?.requirements} must match requirement rows ${requirements.length}`);
	}
	if (report?.status?.incomplete !== incomplete.length) {
		errors.push(`completion incomplete ${report?.status?.incomplete} must match incomplete rows ${incomplete.length}`);
	}
}

function validateCompletionSourceCounters(report, sources) {
	const status = report?.status ?? {};
	const checks = [
		["runtimes", status.runtimes_go, status.runtimes_total, ledgerSummary(sources.runtimeFoundation, "runtimes")],
		["domains", status.domains_go, status.domains_total, ledgerSummary(sources.domainLedger, "domains")],
		["domain_dod", status.domain_dod_go, status.domain_dod_total, ledgerSummary(sources.domainDod, "entries")],
		["gates", status.gates_go, status.gates_total, gateSummary(sources.gateEvidence)],
		["phases", status.phases_go, status.phases_total, ledgerSummary(sources.phaseLedger, "phases")],
	];
	for (const [label, actualGo, actualTotal, expected] of checks) {
		if (actualGo !== expected.go || actualTotal !== expected.total) {
			errors.push(`completion ${label} ${actualGo}/${actualTotal} must match source ${expected.go}/${expected.total}`);
		}
	}
}

function validateCompletionRiskClosure(report, register) {
	const riskClosure = (report?.requirements ?? []).find((entry) => entry.id === "risk-closure");
	const risks = Array.isArray(register?.risks) ? register.risks : [];
	const pending = risks.filter((risk) => risk.status === "pending").length;
	const expected = risks.length > 0 && pending === 0 ? "complete" : "blocked";
	if (!riskClosure) errors.push("completion audit must include risk-closure requirement");
	else if (riskClosure.status !== expected) {
		errors.push(`risk-closure ${riskClosure.status} must be ${expected} for ${pending} pending risk(s)`);
	}
}

function ledgerSummary(ledger, key) {
	const rows = Array.isArray(ledger?.[key]) ? ledger[key] : [];
	return {
		go: rows.filter((row) => row.decision === "go").length,
		total: rows.length,
	};
}

function gateSummary(gates) {
	const rows = Array.isArray(gates?.gates) ? gates.gates : [];
	return {
		go: rows.filter((row) => row.decision === "go").length,
		total: rows.length,
	};
}

function validateBacklogCounters(report) {
	const tasks = Array.isArray(report?.tasks) ? report.tasks : [];
	const openTasks = tasks.filter((task) => task.status !== "complete").length;
	const blockingItems = tasks.reduce((sum, task) => sum + task.blocking_items, 0);
	if (report?.status?.open_tasks !== openTasks) {
		errors.push(`backlog open_tasks ${report?.status?.open_tasks} must match task rows ${openTasks}`);
	}
	if (report?.status?.blocking_items !== blockingItems) {
		errors.push(`backlog blocking_items ${report?.status?.blocking_items} must match task total ${blockingItems}`);
	}
}

function validateLiveEvidenceCounters(report) {
	const requirements = Array.isArray(report?.requirements) ? report.requirements : [];
	const instanceCount = requirements.reduce((sum, requirement) => sum + (requirement.instances?.length ?? 0), 0);
	const missingRequirements = requirements.filter((requirement) => !requirement.ready).length;
	const missingItems = requirements.reduce((sum, requirement) => sum + (requirement.missing_evidence?.length ?? 0), 0);
	if (report?.status?.instance_count !== instanceCount) errors.push(`live evidence instance_count ${report?.status?.instance_count} must match instance rows ${instanceCount}`);
	if (report?.status?.requirements !== requirements.length) errors.push(`live evidence requirements ${report?.status?.requirements} must match requirement rows ${requirements.length}`);
	if (report?.status?.missing_requirements !== missingRequirements) errors.push(`live evidence missing_requirements ${report?.status?.missing_requirements} must match not-ready rows ${missingRequirements}`);
	if (report?.status?.missing_evidence_items !== missingItems) errors.push(`live evidence missing_evidence_items ${report?.status?.missing_evidence_items} must match missing evidence rows ${missingItems}`);
	if (missingRequirements > 0 && missingItems < missingRequirements) errors.push(`live evidence missing_evidence_items ${missingItems} must cover ${missingRequirements} missing requirements`);
}

function validateReadinessLiveEvidence(report, missingRequirements) {
	const sources = Array.isArray(report?.sources) ? report.sources : [];
	const blockers = Array.isArray(report?.blockers) ? report.blockers : [];
	const source = sources.find((row) => row.id === "live_evidence");
	const blocker = blockers.find((row) => row.area === "live_evidence");
	if (!source) {
		errors.push("readiness must include live_evidence source row");
		return;
	}
	if (source.source !== paths.liveEvidence) errors.push(`readiness live_evidence source must be ${paths.liveEvidence}`);
	if (source.pending !== missingRequirements) {
		errors.push(`readiness live_evidence pending ${source.pending} must match missing live evidence ${missingRequirements}`);
	}
	if (missingRequirements > 0 && !blocker) errors.push("readiness live_evidence blocker is required while live evidence is missing");
	if (blocker?.proof !== "node tools/migration/live-evidence-instances.mjs --strict") {
		errors.push(`readiness live_evidence proof ${blocker?.proof} must be node tools/migration/live-evidence-instances.mjs --strict`);
	}
}

if (errors.length > 0) {
	console.error("Migration status consistency checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log(
	`Migration status consistency: ok (${readinessStatus}, ${blockers} blockers, ${missingLiveEvidence} missing live evidence requirements)`,
);
