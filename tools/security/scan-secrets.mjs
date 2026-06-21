#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";

const skippedPathParts = new Set([
	".agents",
	".git",
	".nx",
	"coverage",
	"dist",
	"node_modules",
	"target",
	"vendor",
]);

const skippedFiles = new Set([
	"pnpm-lock.yaml",
	"Cargo.lock",
]);

const textExtensions = new Set([
	".cjs",
	".css",
	".env",
	".html",
	".json",
	".js",
	".jsx",
	".md",
	".mjs",
	".rs",
	".sh",
	".sql",
	".toml",
	".ts",
	".tsx",
	".yaml",
	".yml",
]);

const secretPatterns = [
	{ name: "AWS access key", pattern: /\bAKIA[0-9A-Z]{16}\b/g },
	{ name: "GitHub token", pattern: /\bgh[pousr]_[A-Za-z0-9_]{36,255}\b/g },
	{ name: "Stripe live key", pattern: /\b(?:sk|rk|pk)_live_[A-Za-z0-9]{16,}\b/g },
	{ name: "Slack token", pattern: /\bxox[baprs]-[A-Za-z0-9-]{20,}\b/g },
	{ name: "private key block", pattern: /-----BEGIN (?:RSA |EC |OPENSSH |PGP )?PRIVATE KEY-----/g },
];

function trackedAndUntrackedFiles() {
	const output = execFileSync("git", ["ls-files", "--cached", "--others", "--exclude-standard"], {
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
	});
	return output.split("\n").filter(Boolean);
}

function isScannable(path) {
	if (skippedFiles.has(path)) return false;
	if (path.split("/").some((part) => skippedPathParts.has(part))) return false;
	const dot = path.lastIndexOf(".");
	return dot >= 0 && textExtensions.has(path.slice(dot));
}

const findings = [];

for (const path of trackedAndUntrackedFiles()) {
	if (!isScannable(path) || !existsSync(path)) continue;
	const content = readFileSync(path, "utf8");

	for (const { name, pattern } of secretPatterns) {
		pattern.lastIndex = 0;
		const matches = [...content.matchAll(pattern)];
		for (const match of matches) {
			const line = content.slice(0, match.index).split("\n").length;
			findings.push(`${path}:${line}: ${name}`);
		}
	}
}

if (findings.length > 0) {
	console.error("Secret scan failed:");
	for (const finding of findings) {
		console.error(`- ${finding}`);
	}
	process.exit(1);
}

console.log("Secret scan: ok");
