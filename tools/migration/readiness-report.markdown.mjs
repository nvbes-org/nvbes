export function serializeReadinessMarkdown(report) {
	const lines = [
		"# Migration Readiness Report",
		"",
		"## Status",
		"",
		`- production_cutover: ${report.status.production_cutover}`,
		`- blocking_areas: ${report.status.blocking_areas}`,
		`- blocking_items: ${report.status.blocking_items}`,
		`- live_evidence_missing_requirements: ${report.status.live_evidence_missing_requirements}`,
		`- live_evidence_missing_items: ${report.status.live_evidence_missing_items}`,
		"",
		"## Rules",
		"",
		"- `production_cutover: go` requires zero blocking source rows.",
		"- `blocking_areas` must match the generated blocker list.",
		"- `blocking_items` must equal the sum of all blocker counts.",
		"- Every blocker must reference a source row with the same blocking count.",
		"- Every source and blocker must carry a proof command that references an existing package script or migration tool.",
		"- Source rows must be unique, complete, path-matched and use non-negative counters.",
		"- Generation provenance must identify write and strict cutover commands.",
		"",
		"## Sources",
		"",
		"| Area | Source | Total | Blocking | Proof |",
		"|---|---|---:|---:|---|",
	];
	for (const source of report.sources) {
		lines.push(`| ${source.id} | \`${source.source}\` | ${source.total} | ${source.pending} | \`${source.proof}\` |`);
	}
	lines.push("", "## Blockers", "");
	if (report.blockers.length === 0) {
		lines.push("- none");
	} else {
		for (const blocker of report.blockers) {
			lines.push(`- ${blocker.area}: ${blocker.blocking_items} blocking item(s) in \`${blocker.source}\`; proof: \`${blocker.proof}\``);
		}
	}
	lines.push("", "## Regeneration", "", "```bash", "pnpm check:migration-readiness-report", "tools/migration/readiness-report.mjs --write", "```", "");
	return `${lines.join("\n")}`;
}
