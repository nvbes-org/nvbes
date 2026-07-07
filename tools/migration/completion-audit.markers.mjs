export function structuredMarkerCount(content) {
	return statusSummaryMarkerCount(content) + tableCellMarkerCount(content);
}

function statusSummaryMarkerCount(content) {
	let count = 0;
	for (const line of content.split("\n")) {
		const match = line.match(/^- `([^`]+)`: (\d+)$/);
		if (!match) continue;
		const [, key, value] = match;
		if (summaryKeyContainsMarker(key)) count += Number(value);
	}
	return count;
}

function summaryKeyContainsMarker(key) {
	const normalized = key.toLowerCase().replaceAll("_", "-");
	return [
		"no-go",
		"pending",
		"blocking",
		"not-run",
		"not-scheduled",
		"none",
	].some((marker) => normalized.includes(marker));
}

function tableCellMarkerCount(content) {
	let count = 0;
	for (const line of content.split("\n")) {
		if (!line.startsWith("|") || !line.endsWith("|") || isSeparatorRow(line)) continue;
		const cells = line.split("|").slice(1, -1).map((cell) => cell.trim().toLowerCase());
		for (const cell of cells) if (isBlockingMarkerCell(cell)) count += 1;
	}
	return count;
}

function isSeparatorRow(line) {
	return /^\|[\s|:-]+\|$/.test(line);
}

function isBlockingMarkerCell(cell) {
	return [
		"no-go",
		"pending",
		"blocking",
		"not run",
		"not scheduled",
		"none",
	].includes(cell) || cell.startsWith("no-go ");
}
