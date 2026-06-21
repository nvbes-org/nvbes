function evidenceCell(evidence) {
	return evidence.map((item) => `\`${item}\``).join("<br>");
}

export function serializeCompletionMarkdown(audit) {
	const lines = [
		"# Migration Completion Audit",
		"",
		"## Status",
		"",
		`- objective: ${audit.status.objective}`,
		`- requirements: ${audit.status.requirements}`,
		`- incomplete: ${audit.status.incomplete}`,
		`- readiness: ${audit.status.readiness}`,
		`- readiness_blocking_items: ${audit.status.readiness_blocking_items}`,
		`- runtimes_go: ${audit.status.runtimes_go}/${audit.status.runtimes_total}`,
		`- domains_go: ${audit.status.domains_go}/${audit.status.domains_total}`,
		`- domain_dod_go: ${audit.status.domain_dod_go}/${audit.status.domain_dod_total}`,
		`- gates_go: ${audit.status.gates_go}/${audit.status.gates_total}`,
		`- phases_go: ${audit.status.phases_go}/${audit.status.phases_total}`,
		`- live_evidence_decision: ${audit.status.live_evidence_decision}`,
		`- live_evidence_missing_requirements: ${audit.status.live_evidence_missing_requirements}`,
		`- live_evidence_missing_items: ${audit.status.live_evidence_missing_items}`,
		"",
		"## Rules",
		"",
		"- `requirements` must match the generated requirement rows.",
		"- `incomplete` must equal rows that are not `complete` or `control-active`.",
		"- `objective: complete` requires zero incomplete rows, readiness `go`, live evidence `go`, and zero blockers.",
		"- `objective: complete` requires all runtime, domain, DoD, gate and phase counters to be fully `go`.",
		"- Every requirement must carry source, requirement text, evidence and a proof command.",
		"- Generation provenance must identify write and strict completion commands.",
		"",
		"## Requirements",
		"",
		"| ID | Source | Requirement | Status | Evidence | Proof |",
		"|---|---|---|---:|---|---|",
	];
	for (const requirement of audit.requirements) {
		lines.push(
			`| ${requirement.id} | ${requirement.source} | ${requirement.requirement} | ${requirement.status} | ${evidenceCell(requirement.evidence)} | \`${requirement.proof}\` |`,
		);
	}
	lines.push("", "## Regeneration", "", "```bash", "pnpm check:migration-completion-audit", "tools/migration/completion-audit.mjs --write", "```", "");
	return lines.join("\n");
}
