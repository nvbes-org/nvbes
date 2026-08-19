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
	if (!pattern.test(readFileSync(path, "utf8")))
		errors.push(`${path}: missing ${label}`);
}

requirePath("pnpm-lock.yaml");
requirePath("Cargo.lock");
requirePath("deny.toml");
requirePath(".osv-scanner.toml");
requirePath(".github/dependabot.yml");
requirePath(".github/workflows-archive/security.yml");
requireIncludes(
	"package.json",
	/"packageManager":\s*"pnpm@/,
	"pinned pnpm packageManager",
);
requireIncludes("Cargo.toml", /\[workspace\]/, "Cargo workspace");
requireIncludes("deny.toml", /^\[advisories\]/m, "cargo-deny advisory policy");
requireIncludes("deny.toml", /^\[licenses\]/m, "cargo-deny license policy");
requireIncludes("deny.toml", /^\[bans\]/m, "cargo-deny ban policy");
requireIncludes("deny.toml", /^\[sources\]/m, "cargo-deny source policy");
requireIncludes(
	".github/workflows-archive/security.yml",
	/google\/osv-scanner-action\/osv-scanner-action@[0-9a-f]{40}/,
	"full-SHA-pinned OSV scanner",
);
requireIncludes(
	".github/workflows-archive/security.yml",
	/EmbarkStudios\/cargo-deny-action@[0-9a-f]{40}/,
	"full-SHA-pinned cargo-deny",
);

if (errors.length > 0) {
	console.error("Dependency coverage checks failed:");
	for (const error of errors) console.error(`- ${error}`);
	process.exit(1);
}

console.log("Dependency coverage: ok");
