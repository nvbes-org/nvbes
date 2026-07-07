import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const dashed = (...parts) => parts.join("-");
const spaced = (...parts) => parts.join(" ");
const host = (...parts) => `${parts.join(".")}.fr`;

const forbiddenRuntimeNames = [
	dashed("billing", "api"),
	dashed("drive", "api"),
	dashed("developer", "web"),
	dashed("identity", "api"),
	dashed("identity", "web"),
	dashed("drive", "web"),
	dashed("internal", "admin"),
	dashed("internal", "admin", "web"),
	dashed("gateway", "graphql"),
	dashed("identity", "worker"),
	dashed("drive", "worker"),
	dashed("cloud", "bff"),
];

const forbiddenRuntimeLabelValues = [
	spaced("Billing", "API"),
	spaced("Drive", "API"),
	spaced("Developer", "web"),
	spaced("Identity", "API"),
	spaced("Identity", "web"),
	spaced("Drive", "web"),
	spaced("Internal", "Admin", "web"),
	spaced("Gateway", "GraphQL"),
	spaced("Identity", "worker"),
	spaced("Drive", "worker"),
];

const forbiddenRuntimeLabels = forbiddenRuntimeLabelValues.map((label) => [
	new RegExp(`\\b${label}\\b`, "g"),
	label,
]);

const forbiddenRuntimeHosts = [host("identity", "nvbes"), host("drive", "nvbes")];

const scannedRoots = [
	".github",
	"apps",
	"contracts",
	"deploy",
	"docs",
	"infrastructure",
	"libs",
	"scripts",
	"tools",
];

const scannedRootFiles = [
	"Cargo.toml",
	"package.json",
	"pnpm-workspace.yaml",
	"nx.json",
	"README.md",
];

const textExtensions = new Set([
	".alloy",
	".env",
	".json",
	".md",
	".mjs",
	".rs",
	".sh",
	".sql",
	".tf",
	".toml",
	".ts",
	".tsx",
	".yaml",
	".yml",
]);

const skippedDirs = new Set([".git", ".nx", "coverage", "dist", "node_modules", "target"]);
const allowedLegacyEvidenceFiles = new Set([
	"docs/migration/account-cloud-big-bang.inventory.md",
]);

function normalizePath(value) {
	return value.replaceAll("\\", "/");
}

function extensionFor(path) {
	const basename = path.split("/").at(-1) ?? "";
	if (basename.startsWith(".env")) return ".env";
	const dot = basename.lastIndexOf(".");
	return dot >= 0 ? basename.slice(dot) : "";
}

function shouldScan(path) {
	if (allowedLegacyEvidenceFiles.has(path)) return false;
	if (path.split("/").some((part) => skippedDirs.has(part))) return false;
	return textExtensions.has(extensionFor(path));
}

function walk(dir, results = []) {
	if (!existsSync(dir)) return results;
	for (const entry of readdirSync(dir)) {
		if (skippedDirs.has(entry)) continue;
		const path = join(dir, entry);
		let stat;
		try {
			stat = statSync(path);
		} catch (error) {
			if (error.code === "ENOENT") continue;
			throw error;
		}
		if (stat.isDirectory()) {
			walk(path, results);
		} else {
			const relativePath = normalizePath(relative(process.cwd(), path));
			if (shouldScan(relativePath)) results.push(relativePath);
		}
	}
	return results;
}

function lineMatches(content, path) {
	const matches = [];
	const lines = content.split(/\r?\n/);
	for (let index = 0; index < lines.length; index += 1) {
		const line = lines[index];
		for (const name of forbiddenRuntimeNames) {
			if (line.includes(name)) matches.push(`${path}:${index + 1}: forbidden legacy runtime name ${name}`);
		}
		for (const host of forbiddenRuntimeHosts) {
			if (line.includes(host)) matches.push(`${path}:${index + 1}: forbidden legacy runtime host ${host}`);
		}
		for (const [pattern, label] of forbiddenRuntimeLabels) {
			if (pattern.test(line)) matches.push(`${path}:${index + 1}: forbidden legacy runtime label ${label}`);
			pattern.lastIndex = 0;
		}
	}
	return matches;
}

export function checkLegacyRuntimeNames(errors) {
	const files = [
		...scannedRootFiles.filter((path) => existsSync(path)),
		...scannedRoots.flatMap((root) => walk(root)),
	];
	for (const file of files) {
		errors.push(...lineMatches(readFileSync(file, "utf8"), file));
	}
}
