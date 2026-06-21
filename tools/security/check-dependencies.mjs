#!/usr/bin/env node
import { existsSync, readFileSync } from "node:fs";

const errors = [];

function requirePath(path) {
	if (!existsSync(path)) errors.push(`${path}: missing`);
}

function requireIncludes(path, pattern, label) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return;
	}
	if (!pattern.test(readFileSync(path, "utf8"))) errors.push(`${path}: missing ${label}`);
}

requirePath("pnpm-lock.yaml");
requirePath("Cargo.lock");
requirePath("go.mod");
requirePath("pyproject.toml");
requireIncludes("package.json", /"packageManager":\s*"pnpm@/, "pinned pnpm packageManager");
requireIncludes("Cargo.toml", /\[workspace\]/, "Cargo workspace");
requireIncludes("go.mod", /^module github\.com\/nvbes\/nvbes/m, "nvbes Go module");
requireIncludes("pyproject.toml", /^\[project\]/m, "Python project metadata");

if (errors.length > 0) {
	console.error("Dependency coverage checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log("Dependency coverage: ok");
