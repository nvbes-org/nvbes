function evidenceCell(evidence) {
	if (evidence === "pending") return "pending";
	return evidence
		.split(";")
		.map((entry) => entry.trim())
		.filter(Boolean)
		.map((entry) => (/^(apps|contracts|deploy|docs|infrastructure|libs|scripts|tools)\//.test(entry) ? `\`${entry}\`` : entry))
		.join("<br>");
}

export function serializeRiskMarkdown(register) {
	const lines = [
		"# Migration Risk Register",
		"",
		"## Status",
		"",
		"- Not signed for cutover while any risk remains `pending`.",
		"",
		`- risks: ${register.summary.risks}`,
		`- pending: ${register.summary.pending}`,
		`- mitigated: ${register.summary.mitigated}`,
		`- accepted: ${register.summary.accepted}`,
		`- removed: ${register.summary.removed}`,
		"",
		"## Risks",
		"",
		"| Risk | Owner | Severity | Status | Evidence | Cutover impact |",
		"|---|---|---|---|---|---|",
	];
	for (const risk of register.risks) {
		lines.push(`| ${risk.risk} | ${risk.owner} | ${risk.severity} | ${risk.status} | ${evidenceCell(risk.evidence)} | ${risk.cutover_impact} |`);
	}
	lines.push(
		"",
		"## Rules",
		"",
		"- Every blocking risk from the blueprint must have an owner.",
		"- Every blocking risk must have mitigation evidence before cutover.",
		"- Every `mitigated` or `accepted` risk file evidence path must still exist in the repository.",
		"- A risk can pass cutover only as `mitigated`, `accepted` by owner, or `removed` from scope.",
		"- `pending` risks are no-go.",
		"- Pending risks must expose the evidence required to clear the no-go state.",
		"- Generated Markdown must expose each risk owner, severity, status, evidence and cutover impact.",
		"- Generation provenance must identify the blueprint source, write command and strict cutover command.",
		"",
		"## Review Rules",
		"",
		"| Status | Meaning | Cutover impact |",
		"|---|---|---|",
		"| pending | owner or evidence missing | no-go |",
		"| mitigated | mitigation implemented and evidenced | allowed |",
		"| accepted | owner accepts residual risk | allowed with signed impact |",
		"| removed | risk source removed from scope | allowed |",
		"",
		"## Regeneration",
		"",
		"```bash",
		"tools/migration/risk-register.mjs --write",
		"pnpm check:migration-risk-register",
		"```",
		"",
		"Before a production cutover, run the strict gate:",
		"",
		"```bash",
		"tools/migration/risk-register.mjs --strict",
		"```",
		"",
	);
	return lines.join("\n");
}
