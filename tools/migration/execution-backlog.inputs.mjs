import { existsSync, readFileSync } from "node:fs";

const inputs = {
	readiness: "docs/migration/readiness-report.generated.json",
	completion: "docs/migration/completion-audit.generated.json",
	communication: "docs/migration/communication-plan.md",
	cutoverChecklist: "docs/migration/cutover-checklist.md",
	cutoverJournal: "docs/migration/cutover-journal.md",
	liveEvidence: "docs/migration/live-evidence-instances.generated.json",
	phases: "docs/migration/phase-ledger.generated.json",
	backupRestore: "docs/migration/backup-restore-manifest.md",
	decommission: "docs/migration/decommission-manifest.md",
	gates: "docs/migration/gate-evidence.generated.json",
	observability: "docs/migration/observability-readiness.md",
	ownerSignoffs: "docs/migration/owner-signoff-matrix.md",
	postMigrationAudit: "docs/migration/post-migration-audit.md",
	rehearsals: "docs/migration/rehearsal-ledger.md",
	reconciliationReport: "docs/migration/reconciliation-report.md",
	risks: "docs/migration/risk-register.generated.json",
	rollbackReport: "docs/migration/rollback-report.md",
	reconciliation: "docs/migration/reconciliation.template.json",
	rejects: "docs/migration/rejects.md",
	releaseFreeze: "docs/migration/release-freeze-manifest.md",
	snapshots: "docs/migration/snapshot-manifest.md",
	smokeTests: "docs/migration/smoke-test-manifest.md",
	targetStructure: "docs/migration/target-structure.generated.json",
	v2DebtReview: "docs/migration/v2-debt-review.md",
};

export function loadBacklogInputs(errors) {
	const state = {
		readiness: readJson(inputs.readiness, errors),
		completion: readJson(inputs.completion, errors),
		liveEvidence: readJson(inputs.liveEvidence, errors),
	};
	return {
		...state,
		blockerSources: {
			backupRestore: readText(inputs.backupRestore, errors),
			communication: readText(inputs.communication, errors),
			cutoverChecklist: readText(inputs.cutoverChecklist, errors),
			cutoverJournal: readText(inputs.cutoverJournal, errors),
			decommission: readText(inputs.decommission, errors),
			phases: readJson(inputs.phases, errors),
			gates: readJson(inputs.gates, errors),
			liveEvidence: state.liveEvidence,
			observability: readText(inputs.observability, errors),
			ownerSignoffs: readText(inputs.ownerSignoffs, errors),
			postMigrationAudit: readText(inputs.postMigrationAudit, errors),
			rehearsals: readText(inputs.rehearsals, errors),
			reconciliationReport: readText(inputs.reconciliationReport, errors),
			risks: readJson(inputs.risks, errors),
			rollbackReport: readText(inputs.rollbackReport, errors),
			reconciliation: readJson(inputs.reconciliation, errors),
			rejects: readText(inputs.rejects, errors),
			releaseFreeze: readText(inputs.releaseFreeze, errors),
			snapshots: readText(inputs.snapshots, errors),
			smokeTests: readText(inputs.smokeTests, errors),
			targetStructure: readJson(inputs.targetStructure, errors),
			v2DebtReview: readText(inputs.v2DebtReview, errors),
		},
	};
}

function readJson(path, errors) {
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

function readText(path, errors) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return "";
	}
	return readFileSync(path, "utf8");
}
