#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import {
	cpSync,
	existsSync,
	mkdirSync,
	readFileSync,
	rmSync,
	writeFileSync,
} from "node:fs";
import { dirname, join, relative } from "node:path";

const args = process.argv.slice(2);
const outIndex = args.indexOf("--out");
const outputDir = outIndex >= 0 ? args[outIndex + 1] : "dist/nvbes-oss";
const dryRun = args.includes("--dry-run");

execFileSync(process.execPath, ["tools/oss-export/checks.mjs"], {
	stdio: "inherit",
});

const manifest = JSON.parse(
	readFileSync("tools/oss-export/manifest.json", "utf8"),
);
const explicitCopySources = new Set(
	manifest.copy.map((mapping) => mapping.from),
);
const copyPlan = [
	...manifest.copy,
	...manifest.include
		.filter((source) => !explicitCopySources.has(source))
		.map((source) => ({ from: source, to: source })),
];

if (dryRun) {
	for (const mapping of copyPlan) {
		console.log(`${mapping.from} -> ${join(outputDir, mapping.to)}`);
	}
	process.exit(0);
}

rmSync(outputDir, { recursive: true, force: true });
mkdirSync(outputDir, { recursive: true });

function normalizePath(value) {
	return value.replaceAll("\\", "/");
}

function isExcludedByManifest(path) {
	const normalized = normalizePath(path);
	return manifest.exclude.some((entry) => {
		const exclude = normalizePath(entry).replace(/\/+$/, "");
		if (exclude.includes("*")) {
			return normalized.startsWith(exclude.slice(0, exclude.indexOf("*")));
		}
		return normalized === exclude || normalized.startsWith(`${exclude}/`);
	});
}

function shouldCopy(source) {
	const rel = normalizePath(relative(process.cwd(), source));
	const parts = rel.split("/");

	if (
		parts.some((part) =>
			[".git", ".nx", "coverage", "dist", "node_modules", "target"].includes(
				part,
			),
		)
	) {
		return false;
	}

	return !isExcludedByManifest(rel);
}

for (const mapping of copyPlan) {
	if (!existsSync(mapping.from)) continue;
	const target = join(outputDir, mapping.to);
	mkdirSync(dirname(target), { recursive: true });
	cpSync(mapping.from, target, {
		recursive: true,
		dereference: false,
		errorOnExist: false,
		force: true,
		filter: shouldCopy,
	});
}

rewriteCargoWorkspace();
rewriteProviderBaseline();

function rewriteCargoWorkspace() {
	const cargoPath = join(outputDir, "Cargo.toml");
	if (!existsSync(cargoPath)) return;

	const content = readFileSync(cargoPath, "utf8");
	const rewritten = content.replace(
		/members\s*=\s*\[([\s\S]*?)\]/m,
		(_match, body) => {
			const lines = body.split("\n").filter((line) => {
				const member = line.match(/"([^"]+)"/)?.[1];
				return !member || !isExcludedByManifest(member);
			});
			return `members = [${lines.join("\n")}]`;
		},
	);

	writeFileSync(cargoPath, rewritten);
}

function rewriteProviderBaseline() {
	const baselinePath = join(outputDir, "scripts/oss-provider-baseline.json");
	if (!existsSync(baselinePath)) return;

	const blockedTerms = [
		"stripe",
		"sentry",
		"scaleway",
		"cloudflare",
		"posthog",
		"sendgrid",
		"mailgun",
		"postmark",
	];
	const baseline = JSON.parse(readFileSync(baselinePath, "utf8"));
	const knownProviderFiles = baseline.knownProviderFiles.filter((filePath) => {
		const exportedPath = join(outputDir, filePath);
		if (!existsSync(exportedPath)) return false;

		const content = readFileSync(exportedPath, "utf8").toLowerCase();
		return blockedTerms.some((term) => content.includes(term));
	});

	writeFileSync(
		baselinePath,
		`${JSON.stringify({ ...baseline, knownProviderFiles }, null, 2)}\n`,
	);
}

console.log(`OSS export written to ${outputDir}`);
