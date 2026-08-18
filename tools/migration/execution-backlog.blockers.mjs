import { blockingMarkdownDecisionDetails } from "./markdown-decision-rows.mjs";

export function readinessBlockingDetails(blocker, sources) {
	const details = detailsByArea(blocker.area, sources);
	if (details.length === blocker.blocking_items) return details;
	return fallbackDetails(blocker);
}

function detailsByArea(area, sources) {
	if (area === "phases") return pendingPhases(sources.phases);
	if (area === "gates") return pendingGates(sources.gates, "gate");
	if (area === "risks") return pendingRisks(sources.risks);
	if (area === "target_structure") return missingTargetStructure(sources.targetStructure);
	if (area === "reconciliation_template") return reconciliationTemplate(sources.reconciliation);
	if (area === "live_evidence") return missingLiveEvidence(sources.liveEvidence);
	if (markdownSource(area, sources)) return blockingMarkdownDecisionDetails(markdownSource(area, sources));
	return [];
}

function pendingPhases(phases) {
	return (phases?.phases ?? [])
		.filter((phase) => phase.status !== "passed" && phase.status !== "accepted")
		.map((phase) => `${phase.phase} ${phase.title}: ${phase.status}/${phase.decision} (${phase.owner})`);
}

function pendingGates(gates, label) {
	return (gates?.gates ?? [])
		.filter((gate) => gate.status !== "passed" && gate.status !== "accepted")
		.map((gate) => `${gate.gate}: ${label} ${gate.status}/${gate.decision} (${gate.owner}) - ${gate.notes}`);
}

function pendingRisks(register) {
	return (register?.risks ?? [])
		.filter((risk) => risk.status !== "mitigated" && risk.status !== "accepted" && risk.status !== "removed")
		.map((risk) => `${risk.id}: ${risk.status} ${risk.severity} risk owned by ${risk.owner} - ${risk.cutover_impact}`);
}

function missingTargetStructure(targetStructure) {
	return (targetStructure?.entries ?? [])
		.filter((entry) => entry.status !== "present")
		.map((entry) => `${entry.path}: ${entry.status}`);
}

function reconciliationTemplate(reconciliation) {
	if (!reconciliation) return [];
	const blockingDomains = (reconciliation.domains ?? []).filter((domain) => domain.status !== "passed" && domain.status !== "accepted");
	return [
		`${reconciliation.environment}/${reconciliation.snapshot_id}: ${blockingDomains.length} blocking domains, decision ${reconciliation.decision}`,
	];
}

function missingLiveEvidence(liveEvidence) {
	return (liveEvidence?.requirements ?? [])
		.filter((requirement) => !requirement.ready)
		.map((requirement) => `${requirement.id}: ${(requirement.missing_evidence ?? []).join(", ")}`);
}

function fallbackDetails(blocker) {
	return Array.from(
		{ length: blocker.blocking_items },
		(_, index) => `${blocker.area} unresolved item ${index + 1}/${blocker.blocking_items}`,
	);
}

function markdownSource(area, sources) {
	return {
		communication: sources.communication,
		cutover_checklist: sources.cutoverChecklist,
		cutover_journal: sources.cutoverJournal,
		observability: sources.observability,
		owner_signoffs: sources.ownerSignoffs,
		reconciliation_report: sources.reconciliationReport,
		rehearsals: sources.rehearsals,
		rejects: sources.rejects,
		release_freeze: sources.releaseFreeze,
		snapshots: sources.snapshots,
	}[area];
}
