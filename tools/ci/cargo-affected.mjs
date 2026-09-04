#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import { appendFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { selectAffectedCargoPackages } from "./cargo-affected.core.mjs";

const workspace = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

function requiredEnvironment(name) {
	const value = process.env[name];
	if (!value) throw new Error(`${name} must be set`);
	return value;
}

function command(binary, arguments_) {
	return execFileSync(binary, arguments_, {
		cwd: workspace,
		encoding: "utf8",
		stdio: ["ignore", "pipe", "inherit"],
	}).trim();
}

const baseSha = requiredEnvironment("NVBES_CI_BASE_SHA");
const headSha = requiredEnvironment("GITHUB_SHA");
const changedPaths = command("git", [
	"diff",
	"--name-only",
	"--diff-filter=ACMRD",
	baseSha,
	headSha,
])
	.split("\n")
	.filter(Boolean);
const metadata = JSON.parse(
	command("cargo", [
		"metadata",
		"--format-version",
		"1",
		"--locked",
		"--no-deps",
	]),
);
const packages = selectAffectedCargoPackages(metadata, changedPaths, workspace);
if (packages.length === 0)
	throw new Error("Rust change resolved to no Cargo package");
appendFileSync(
	requiredEnvironment("GITHUB_OUTPUT"),
	`packages=${packages.join(",")}\n`,
);
console.log(`Affected Cargo packages: ${packages.join(", ")}`);
