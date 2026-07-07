import { structuredMarkerCount } from "./completion-audit.markers.mjs";

export function validateCompletionMarkerCounter(errors) {
	const fixture = [
		"Prose mentions no-go, pending, blocking, none, not run and not scheduled.",
		"- `pending_rows`: 2",
		"- `no_go_decisions`: 1",
		"- `not_run_rows`: 1",
		"| Item | Decision |",
		"|---|---|",
		"| row | no-go until rehearsed |",
		"| row | pending |",
		"| row | none |",
		"| row | not scheduled |",
		"| row | not run |",
		"| row | blocking |",
	].join("\n");
	const count = structuredMarkerCount(fixture);
	if (count !== 10) errors.push(`completion marker counter regression: expected 10, got ${count}`);
}
