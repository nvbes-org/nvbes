const blockingValues = new Set([
	"blocking",
	"failed",
	"no-go",
	"none",
	"not run",
	"not scheduled",
	"pending",
]);
const actionableHeaders = new Set(["current state", "decision", "status"]);

export function markdownDecisionRows(content) {
	const rows = [];
	let section = "Document";
	for (const table of markdownTables(content)) {
		const headers = table.header.map(normalizeCell);
		const actionableIndexes = headers
			.map((header, index) => actionableHeaders.has(header) ? index : -1)
			.filter((index) => index >= 0);
		const fieldValueTable = headers.length === 2 && headers[0] === "field" && headers[1] === "value";
		if (actionableIndexes.length === 0 && !fieldValueTable) continue;

		for (const cells of table.rows) {
			const states = fieldValueTable ? fieldValueStates(cells) : stateCells(cells, actionableIndexes);
			if (states.length === 0) continue;
			rows.push({
				section,
				label: labelFor(cells),
				states,
				blocking: states.some((state) => isBlockingState(state.value)),
			});
		}
		section = table.nextSection;
	}
	return rows;
}

export function blockingMarkdownDecisionDetails(content) {
	return markdownDecisionRows(content)
		.filter((row) => row.blocking)
		.map((row) => {
			const states = row.states.map((state) => `${state.name}=${state.value}`).join(", ");
			return `${row.section}: ${row.label} (${states})`;
		});
}

function markdownTables(content) {
	const lines = content.split(/\r?\n/);
	const tables = [];
	let section = "Document";
	for (let index = 0; index < lines.length; index += 1) {
		const heading = lines[index].match(/^##+ (.+)$/);
		if (heading) section = heading[1].trim();
		if (!isTableRow(lines[index]) || !isSeparatorRow(lines[index + 1] ?? "")) continue;

		const header = tableCells(lines[index]);
		const rows = [];
		index += 2;
		while (index < lines.length && isTableRow(lines[index])) {
			rows.push(tableCells(lines[index]));
			index += 1;
		}
		const nextSection = section;
		tables.push({ header, rows, nextSection });
		index -= 1;
	}
	return tables;
}

function stateCells(cells, indexes) {
	return indexes
		.map((index) => ({ name: `column:${index + 1}`, value: normalizeCell(cells[index] ?? "") }))
		.filter((state) => state.value.length > 0);
}

function fieldValueStates(cells) {
	const field = normalizeCell(cells[0] ?? "");
	const value = normalizeCell(cells[1] ?? "");
	if (!field || !value) return [];
	if (field.includes("decision") || isBlockingState(value)) return [{ name: field, value }];
	return [];
}

function labelFor(cells) {
	return cells.slice(0, 2).filter(Boolean).join(" / ");
}

function isBlockingState(value) {
	return blockingValues.has(value) || value.startsWith("no-go ");
}

function isTableRow(line) {
	return line.startsWith("|") && line.endsWith("|");
}

function isSeparatorRow(line) {
	return /^\|[\s|:-]+\|$/.test(line);
}

function tableCells(line) {
	return line.split("|").slice(1, -1).map((cell) => cell.trim());
}

function normalizeCell(value) {
	return value.trim().toLowerCase().replaceAll("`", "");
}
