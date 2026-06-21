export const requirements = [
	{
		id: "g4-frontend-signoff",
		scope: "G4 Frontends",
		live_evidence: "critical Playwright journeys and WCAG AA sign-off",
		files: [
			"docs/migration/frontend-experience.generated.json",
			"docs/migration/smoke-test-manifest.md",
		],
		commands: [
			"check:migration-frontend-experience",
			"check:web",
			"check:migration-smoke-tests",
		],
	},
	{
		id: "g5-infra-signoff",
		scope: "G5 Infra",
		live_evidence: "staging rebuild, backup restore and timed rollback proof",
		files: [
			"docs/migration/infra-deploy.generated.json",
			"docs/migration/backup-restore-manifest.md",
			"docs/migration/rollback-report.md",
		],
		commands: [
			"check:migration-infra-deploy",
			"check:migration-backup-restore",
			"check:migration-rollback-report",
		],
	},
	{
		id: "p11-rehearsals",
		scope: "P11/G6 Repetitions migration",
		live_evidence: "three rehearsal runs and two stable reconciliations",
		files: [
			"docs/migration/rehearsal-ledger.md",
			"docs/migration/reconciliation-report.schema.json",
			"docs/migration/reconciliation.template.json",
			"docs/migration/rejects.md",
		],
		commands: [
			"check:migration-rehearsals",
			"check:migration-reconciliation-report",
			"check:migration-rejects",
		],
	},
	{
		id: "p12-cutover",
		scope: "P12/G7 Big Bang cutover",
		live_evidence: "production cutover journal, final reconciliation, smoke and SLO proof",
		files: [
			"docs/migration/cutover-journal.md",
			"docs/migration/cutover-checklist.md",
			"docs/migration/observability-readiness.md",
			"docs/migration/smoke-test-manifest.md",
			"tools/migration/precutover-gate.mjs",
		],
		commands: [
			"check:migration-precutover",
			"check:migration-cutover-journal",
			"check:migration-cutover-checklist",
			"check:migration-observability",
			"check:migration-smoke-tests",
		],
	},
	{
		id: "p13-decommission",
		scope: "P13/G8 Decommission",
		live_evidence: "legacy runtime removed, secrets revoked and post-migration audit approved",
		files: [
			"docs/migration/decommission-manifest.md",
			"docs/migration/post-migration-audit.md",
			"tools/migration/postcutover-gate.mjs",
		],
		commands: [
			"check:migration-postcutover",
			"check:migration-decommission",
			"check:migration-post-migration-audit",
		],
	},
	{
		id: "final-reconciliation",
		scope: "Production reconciliation",
		live_evidence: "executed production reconciliation JSON replacing the template no-go report",
		files: [
			"docs/migration/reconciliation-report.schema.json",
			"docs/migration/reconciliation.template.json",
			"tools/migration/reconcile.mjs",
		],
		commands: [
			"check:migration-reconciliation-report",
			"check:migration-reconciliation-template",
		],
	},
];

export function strictCommandFor(requirement) {
	if (requirement.id === "g4-frontend-signoff") {
		return "pnpm check:migration-frontend-experience && pnpm check:web && pnpm check:migration-smoke-tests -- --strict";
	}
	if (requirement.id === "g5-infra-signoff") {
		return "pnpm check:migration-infra-deploy && pnpm check:migration-backup-restore -- --strict && pnpm check:migration-rollback-report -- --strict";
	}
	if (requirement.id === "p11-rehearsals") {
		return "pnpm check:migration-rehearsals -- --strict && node tools/migration/reconcile.mjs --env staging --report docs/migration/reconciliation.<run>.json && pnpm check:migration-rejects -- --strict";
	}
	if (requirement.id === "p12-cutover") {
		return "pnpm check:migration-precutover -- --env production --reconciliation-report docs/migration/reconciliation.<run>.json";
	}
	if (requirement.id === "p13-decommission") return "pnpm check:migration-postcutover";
	if (requirement.id === "final-reconciliation") {
		return "node tools/migration/reconcile.mjs --env production --report docs/migration/reconciliation.<run>.json";
	}
	return requirement.commands.map((command) => `pnpm ${command} -- --strict`).join(" && ");
}
