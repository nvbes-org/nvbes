#!/usr/bin/env node

import { execFileSync } from "node:child_process";
import {
	appendFileSync,
	existsSync,
	lstatSync,
	mkdirSync,
	readFileSync,
	symlinkSync,
} from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
	isTrustedPush,
	isUsableCommitSha,
	selectTypeScriptProjects,
	trustedCacheDirectory,
} from "./nx-cache-manager.core.mjs";

const workspace = join(dirname(fileURLToPath(import.meta.url)), "..", "..");

function requiredEnvironment(name) {
	const value = process.env[name];
	if (value === undefined || value.length === 0) {
		throw new Error(`${name} must be set`);
	}
	return value;
}

function git(...arguments_) {
	return execFileSync("git", arguments_, {
		cwd: workspace,
		encoding: "utf8",
		stdio: ["ignore", "pipe", "inherit"],
	}).trim();
}

function commitExists(sha) {
	try {
		git("cat-file", "-e", `${sha}^{commit}`);
		return true;
	} catch {
		return false;
	}
}

function resolveBaseSha(candidate, eventName, headSha) {
	if (isUsableCommitSha(candidate) && commitExists(candidate)) return candidate;

	if (eventName !== "workflow_dispatch") {
		const mergeBase = git("merge-base", headSha, "origin/main");
		if (mergeBase !== headSha) return mergeBase;
	}

	const parent = `${headSha}^`;
	return commitExists(parent) ? git("rev-parse", parent) : headSha;
}

function nxJson(...arguments_) {
	const output = execFileSync("pnpm", ["exec", "nx", ...arguments_], {
		cwd: workspace,
		encoding: "utf8",
		env: { ...process.env, NX_NO_CLOUD: "true" },
		stdio: ["ignore", "pipe", "inherit"],
	});
	return JSON.parse(output);
}

function configureTrustedCache(environment) {
	if (!isTrustedPush(environment.eventName, environment.ref)) {
		return "ephemeral";
	}

	const packageJson = JSON.parse(
		readFileSync(join(workspace, "package.json"), "utf8"),
	);
	const persistentDirectory = trustedCacheDirectory({
		runnerToolCache: environment.runnerToolCache,
		runnerOs: environment.runnerOs,
		runnerArch: environment.runnerArch,
		nxVersion: packageJson.devDependencies.nx,
	});
	const workspaceCache = join(workspace, ".nx", "cache");

	mkdirSync(persistentDirectory, { recursive: true });
	mkdirSync(dirname(workspaceCache), { recursive: true });
	if (existsSync(workspaceCache)) {
		const existing = lstatSync(workspaceCache);
		if (!existing.isSymbolicLink()) {
			throw new Error(`${workspaceCache} must not exist before cache setup`);
		}
		return "persistent-trusted";
	}
	symlinkSync(persistentDirectory, workspaceCache, "dir");
	return "persistent-trusted";
}

const environment = {
	eventName: requiredEnvironment("GITHUB_EVENT_NAME"),
	ref: requiredEnvironment("GITHUB_REF"),
	headSha: requiredEnvironment("GITHUB_SHA"),
	runnerToolCache: requiredEnvironment("RUNNER_TOOL_CACHE"),
	runnerOs: requiredEnvironment("RUNNER_OS"),
	runnerArch: requiredEnvironment("RUNNER_ARCH"),
};
const baseSha = resolveBaseSha(
	process.env.NVBES_CI_BASE_CANDIDATE ?? "",
	environment.eventName,
	environment.headSha,
);
const cacheMode = configureTrustedCache(environment);
const affectedProjects = nxJson(
	"show",
	"projects",
	"--affected",
	`--base=${baseSha}`,
	`--head=${environment.headSha}`,
	"--json",
);
const graph = nxJson("graph", "--print").graph.nodes;
const typeScriptProjects = selectTypeScriptProjects(affectedProjects, graph);
const rustAffected = affectedProjects.includes("rust-workspace");
const githubOutput = requiredEnvironment("GITHUB_OUTPUT");
const githubEnvironment = requiredEnvironment("GITHUB_ENV");

appendFileSync(
	githubOutput,
	[
		`base-sha=${baseSha}`,
		`projects=${JSON.stringify(affectedProjects)}`,
		`typescript-projects=${typeScriptProjects.join(",")}`,
		`rust-affected=${rustAffected}`,
		`cache-mode=${cacheMode}`,
		"",
	].join("\n"),
);
appendFileSync(githubEnvironment, `NVBES_CI_BASE_SHA=${baseSha}\n`);

console.log(
	JSON.stringify({
		baseSha,
		cacheMode,
		affectedProjects,
		typeScriptProjects,
		rustAffected,
	}),
);
