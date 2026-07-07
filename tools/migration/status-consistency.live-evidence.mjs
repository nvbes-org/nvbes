export function validatePacketLiveEvidenceAlignment(packet, liveEvidence, errors) {
	const packetRows = Array.isArray(packet?.requirements) ? packet.requirements : [];
	const liveRows = Array.isArray(liveEvidence?.requirements) ? liveEvidence.requirements : [];
	const liveById = new Map(liveRows.map((row) => [row.id, row]));
	if (packetRows.length !== liveRows.length) {
		errors.push(`cutover packet requirements ${packetRows.length} must match live evidence requirements ${liveRows.length}`);
	}
	for (const packetRow of packetRows) {
		const liveRow = liveById.get(packetRow.id);
		if (!liveRow) {
			errors.push(`live evidence must include cutover packet requirement ${packetRow.id}`);
			continue;
		}
		if (liveRow.scope !== packetRow.scope) errors.push(`${packetRow.id}: live evidence scope must match cutover packet`);
		if (liveRow.strict_command !== packetRow.strict_command) {
			errors.push(`${packetRow.id}: live evidence strict_command must match cutover packet`);
		}
		const expectedLiveState = liveRow.ready ? "ready" : "missing";
		if (packetRow.current?.live_evidence !== expectedLiveState) {
			errors.push(`${packetRow.id}: packet current.live_evidence must match live evidence readiness`);
		}
		if (!sameList(packetRow.current?.missing_evidence, liveRow.missing_evidence ?? [])) {
			errors.push(`${packetRow.id}: packet current.missing_evidence must match live evidence missing rows`);
		}
		const hasMissingEvidenceBlocker = (packetRow.live_blocking_reasons ?? []).includes("live evidence missing");
		if (hasMissingEvidenceBlocker === liveRow.ready) {
			errors.push(`${packetRow.id}: packet live evidence missing blocker must match live evidence readiness`);
		}
	}
	for (const liveRow of liveRows) {
		if (!packetRows.some((packetRow) => packetRow.id === liveRow.id)) {
			errors.push(`cutover packet must include live evidence requirement ${liveRow.id}`);
		}
	}
}

function sameList(actual, expected) {
	return Array.isArray(actual)
		&& actual.length === expected.length
		&& actual.every((value, index) => value === expected[index]);
}
