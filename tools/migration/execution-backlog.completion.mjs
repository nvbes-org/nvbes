export function completionBlockingDetails(item, sources) {
	switch (item.id) {
		case "backup-rollback":
			return [backupRollbackDetail(sources)];
		case "critical-journeys":
			return [smokeTestDetail(sources.smokeTests)];
		case "data-reconciled":
			return [reconciliationDetail(sources.reconciliation)];
		case "gate-evidence":
			return [gateDetail(sources.gates)];
		case "legacy-jobs-stopped":
			return [decommissionDetail(sources.decommission, "legacy jobs")];
		case "legacy-secrets-revoked":
			return [decommissionDetail(sources.decommission, "legacy secrets")];
		case "live-cutover-evidence":
			return [liveEvidenceDetail(sources.liveEvidence)];
		case "no-runtime-legacy":
			return [decommissionDetail(sources.decommission, "legacy runtime")];
		case "no-v2-debt":
			return [v2DebtReviewDetail(sources.v2DebtReview)];
		case "owner-signoffs":
			return [statusSummaryDetail("owner signoffs", sources.ownerSignoffs, ["pending_rows", "no_go_decisions"])];
		case "cutover-checklist":
			return [statusSummaryDetail("cutover checklist", sources.cutoverChecklist, ["pending_rows", "blocking_rows"])];
		case "phase-acceptance":
			return [phaseDetail(sources.phases)];
		case "post-audit-approved":
			return [postAuditDetail(sources.postMigrationAudit)];
		case "rehearsal-evidence":
			return [statusSummaryDetail("rehearsals", sources.rehearsals, ["pending_run_rows", "run_no_go_decisions", "stability_no_go_decisions"])];
		case "snapshot-release-freeze":
			return [
				[
					statusSummaryDetail("snapshots", sources.snapshots, ["restore_no_go_decisions"]),
					statusSummaryDetail("release freeze", sources.releaseFreeze, ["pending_rows", "no_go_decisions"]),
				].join("; "),
			];
		case "communication-observability":
			return [
				[
					statusSummaryDetail("communication", sources.communication, ["pending_audience_rows", "pending_approval_rows", "no_go_decisions"]),
					statusSummaryDetail("observability", sources.observability, ["pending_rows", "no_go_decisions"]),
				].join("; "),
			];
		case "rejects-reconciliation-journal":
			return [
				[
					statusSummaryDetail("rejects", sources.rejects, ["blocking_reject_rows", "no_go_decisions"]),
					statusSummaryDetail("reconciliation report", sources.reconciliationReport, ["blocking_domain_rows", "metadata_no_go_decisions"]),
					statusSummaryDetail("cutover journal", sources.cutoverJournal, ["metadata_no_go_decisions", "timeline_no_go_decisions"]),
				].join("; "),
			];
		case "risk-closure":
			return [riskDetail(sources.risks)];
		case "runbook-explicit":
			return [runbookDetail(sources)];
		default:
			return [item.requirement];
	}
}

function backupRollbackDetail(sources) {
	const backup = statusSummary(sources.backupRestore);
	const rollback = statusSummary(sources.rollbackReport);
	return [
		`backup/restore ${backup.pending_rows ?? "?"} pending rows and ${backup.no_go_decisions ?? "?"} no-go decisions`,
		`rollback ${rollback.blocking_step_rows ?? "?"} blocking steps, ${rollback.none_metadata_rows ?? "?"} empty metadata rows, ${rollback.no_go_metadata_decisions ?? "?"} no-go decision`,
	].join("; ");
}

function smokeTestDetail(content) {
	const status = statusSummary(content);
	return `smoke manifest ${status.pending_journey_rows ?? "?"} pending journeys, ${status.pending_runtime_evidence_rows ?? "?"} pending runtime evidence rows, ${status.no_go_decisions ?? "?"} no-go decisions`;
}

function reconciliationDetail(reconciliation) {
	if (!reconciliation) return "reconciliation report missing";
	const blocking = (reconciliation.domains ?? []).filter((domain) => !["passed", "accepted"].includes(domain.status));
	return `${reconciliation.environment}/${reconciliation.snapshot_id}: ${blocking.length} blocking domains, decision ${reconciliation.decision}`;
}

function gateDetail(gates) {
	const pending = (gates?.gates ?? []).filter((gate) => !["passed", "accepted"].includes(gate.status));
	return `${pending.length} gate(s) pending/no-go: ${pending.map((gate) => gate.gate).join(", ")}`;
}

function decommissionDetail(content, scope) {
	const status = statusSummary(content);
	return `${scope}: decommission has ${status.pending_rows ?? "?"} pending rows and ${status.no_go_decisions ?? "?"} no-go decisions`;
}

function liveEvidenceDetail(liveEvidence) {
	const missing = liveEvidence?.status?.missing_requirements ?? "?";
	const items = liveEvidence?.status?.missing_evidence_items ?? "?";
	return `live evidence ${liveEvidence?.status?.decision ?? "unknown"} with ${missing} missing requirements and ${items} missing evidence items`;
}

function phaseDetail(phases) {
	const pending = (phases?.phases ?? []).filter((phase) => !["passed", "accepted"].includes(phase.status));
	return `${pending.length} phase(s) pending/no-go: ${pending.map((phase) => `${phase.phase} ${phase.title}`).join(", ")}`;
}

function postAuditDetail(content) {
	const status = statusSummary(content);
	return `post-migration audit ${status.pending_signoff_rows ?? "?"} pending sign-offs and ${status.pending_audit_rows ?? "?"} pending audit items`;
}

function riskDetail(register) {
	const risks = (register?.risks ?? []).filter((risk) => !["mitigated", "accepted", "removed"].includes(risk.status));
	return `${risks.length} unresolved risk(s): ${risks.map((risk) => `${risk.id} ${risk.status}`).join(", ")}`;
}

function runbookDetail(sources) {
	return [
		liveEvidenceDetail(sources.liveEvidence),
		gateDetail(sources.gates),
		phaseDetail(sources.phases),
		reconciliationDetail(sources.reconciliation),
	].join("; ");
}

function v2DebtReviewDetail(review) {
	const ownerAcceptance = review.match(/- owner_acceptance: ([^\n]+)/)?.[1]?.trim() ?? "unknown";
	const findings = sectionTableFirstCells(review, "## Findings");
	return `owner_acceptance ${ownerAcceptance}; ${findings.length} V1-required finding(s): ${findings.join("; ")}`;
}

function statusSummaryDetail(label, content, keys) {
	const status = statusSummary(content);
	const details = keys.map((key) => `${status[key] ?? "?"} ${key.replaceAll("_", " ")}`);
	return `${label} ${details.join(", ")}`;
}

function statusSummary(content = "") {
	const entries = [...content.matchAll(/^- `?([a-z0-9_]+)`?: (\d+)/gm)];
	return Object.fromEntries(entries.map(([, key, value]) => [key, Number.parseInt(value, 10)]));
}

function sectionTableFirstCells(content, heading) {
	const start = content.indexOf(heading);
	if (start === -1) return [];
	const section = content.slice(start).split(/\n## /)[0];
	return section
		.split("\n")
		.filter((line) => line.startsWith("|") && !line.includes("---"))
		.map((line) => line.split("|")[1]?.trim())
		.filter((cell) => cell && cell !== "Finding");
}
