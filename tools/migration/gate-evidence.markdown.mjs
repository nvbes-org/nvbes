function evidenceCell(evidence) {
	return evidence.length === 0 ? "none" : evidence.map((item) => `\`${item}\``).join("<br>");
}

export function serializeGateEvidenceMarkdown(register) {
	const lines = [
		"# Migration Gate Evidence",
		"",
		"## Status",
		"",
		"- No gate is approved for production cutover unless its generated row is `passed` or `accepted`, has decision `go`, and carries evidence.",
		"",
		`- gates: ${register.summary.gates}`,
		`- pending: ${register.summary.pending}`,
		`- passed: ${register.summary.passed}`,
		`- accepted: ${register.summary.accepted}`,
		`- failed: ${register.summary.failed}`,
		"",
		"## Rules",
		"",
		"- Every blueprint gate must have an owner.",
		"- Every gate must attach evidence before it can be marked `go`.",
		"- Every `passed` or `accepted` gate evidence path must still exist in the repository.",
		"- A production cutover requires all gates to be `passed` or `accepted`.",
		"- A missing gate, missing evidence or `no-go` decision blocks cutover.",
		"- Pending cutover gates must expose the live evidence packet and notes required to unlock them.",
		"- Generation provenance must identify the blueprint source, write command and strict cutover command.",
		"",
		"## Gates",
		"",
		"| Gate | Owner | Status | Decision | Evidence | Notes |",
		"|---|---|---|---|---|---|",
	];
	for (const gate of register.gates) {
		lines.push(`| ${gate.gate} | ${gate.owner} | ${gate.status} | ${gate.decision} | ${evidenceCell(gate.evidence)} | ${gate.notes} |`);
	}
	lines.push(
		"",
		"## Evidence Types",
		"",
		"| Type | Example |",
		"|---|---|",
		"| command output | `pnpm check`, `cargo check --workspace` |",
		"| report | reconciliation, rollback, post-migration audit |",
		"| checksum | data migration checksum |",
		"| CI link | immutable CI run URL |",
		"| sign-off | owner-approved decision record |",
		"",
		"## Regeneration",
		"",
		"```bash",
		"node tools/migration/gate-evidence.mjs --write",
		"pnpm check:migration-gate-evidence",
		"```",
		"",
		"Before a production cutover, run the strict gate:",
		"",
		"```bash",
		"node tools/migration/gate-evidence.mjs --strict",
		"```",
		"",
	);
	return lines.join("\n");
}
