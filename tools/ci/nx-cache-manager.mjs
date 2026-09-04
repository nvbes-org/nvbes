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
	isContainerScopeAffected,
	isDatabaseScopeAffected,
	isDeliveryScopeAffected,
	isRustToolchainRequired,
	isRustWorkspaceAffected,
	isTrustedPush,
	isUsableCommitSha,
	selectTypeScriptProjects,
	terraformScopes,
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
const changedPaths = git(
	"diff",
	"--name-only",
	"--diff-filter=ACMRD",
	baseSha,
	environment.headSha,
)
	.split("\n")
	.filter(Boolean);
const graph = nxJson("graph", "--print").graph.nodes;
const typeScriptProjects = selectTypeScriptProjects(affectedProjects, graph);
const rustAffected = isRustWorkspaceAffected(changedPaths);
const deliveryScopes = {
	email: isDeliveryScopeAffected(affectedProjects, changedPaths, {
		projects: ["email-worker", "email-ui", "email-scaleway"],
		paths: [
			"apps/email-worker",
			"libs/rust/email",
			"infrastructure/environments/email-production",
			"infrastructure/stacks/email",
		],
	}),
	identity: isDeliveryScopeAffected(affectedProjects, changedPaths, {
		projects: [
			"identity-service",
			"identity-sdk",
			"identity-sdk-backend",
			"identity-sdk-core",
			"identity-sdk-web",
			"identity-client",
		],
		paths: [
			"apps/identity-service",
			"contracts/protobuf/nvbes/identity",
			"infrastructure/environments/identity-production",
		],
	}),
	account: isDeliveryScopeAffected(affectedProjects, changedPaths, {
		projects: ["account-service", "account-client"],
		paths: [
			"apps/account-service",
			"contracts/protobuf/nvbes/account",
			"infrastructure/environments/account-production",
		],
	}),
	billing: isDeliveryScopeAffected(affectedProjects, changedPaths, {
		projects: ["billing-service", "billing-client", "billing"],
		paths: [
			"apps/billing-service",
			"libs/rust/billing",
			"contracts/protobuf/nvbes/billing",
			"infrastructure/environments/billing-production",
		],
	}),
	trustRisk: isDeliveryScopeAffected(affectedProjects, changedPaths, {
		projects: ["trust-risk-service", "trust-risk"],
		paths: [
			"apps/trust-risk-service",
			"infrastructure/environments/trust-risk-production",
		],
	}),
	platform: isDeliveryScopeAffected(affectedProjects, changedPaths, {
		projects: ["platform-operations-service", "platform"],
		paths: [
			"apps/platform-operations-service",
			"libs/rust/platform",
			"infrastructure/environments/platform-operations-production",
		],
	}),
};
const databases = {
	account: isDatabaseScopeAffected(changedPaths, "account"),
	email: isDatabaseScopeAffected(changedPaths, "email"),
};
const rustRequired = isRustToolchainRequired(rustAffected, databases);
const containers = {
	account: isContainerScopeAffected(changedPaths, "apps/account-service"),
	billing: isContainerScopeAffected(changedPaths, "apps/billing-service"),
	email: isContainerScopeAffected(changedPaths, "apps/email-worker"),
	identity: isContainerScopeAffected(changedPaths, "apps/identity-service"),
	platform: isContainerScopeAffected(
		changedPaths,
		"apps/platform-operations-service",
	),
	trustRisk: isContainerScopeAffected(changedPaths, "apps/trust-risk-service"),
};
const terraform = terraformScopes(changedPaths);
const githubOutput = requiredEnvironment("GITHUB_OUTPUT");
const githubEnvironment = requiredEnvironment("GITHUB_ENV");

appendFileSync(
	githubOutput,
	[
		`base-sha=${baseSha}`,
		`projects=${JSON.stringify(affectedProjects)}`,
		`typescript-projects=${typeScriptProjects.join(",")}`,
		`rust-affected=${rustAffected}`,
		`rust-required=${rustRequired}`,
		`email-affected=${deliveryScopes.email}`,
		`identity-affected=${deliveryScopes.identity}`,
		`account-affected=${deliveryScopes.account}`,
		`billing-affected=${deliveryScopes.billing}`,
		`trust-risk-affected=${deliveryScopes.trustRisk}`,
		`platform-affected=${deliveryScopes.platform}`,
		`account-database-affected=${databases.account}`,
		`email-database-affected=${databases.email}`,
		`account-container-affected=${containers.account}`,
		`billing-container-affected=${containers.billing}`,
		`email-container-affected=${containers.email}`,
		`identity-container-affected=${containers.identity}`,
		`platform-container-affected=${containers.platform}`,
		`trust-risk-container-affected=${containers.trustRisk}`,
		`terraform-scopes=${terraform.join(",")}`,
		`terraform-affected=${terraform.length > 0}`,
		...[
			"account",
			"billing",
			"email",
			"identity",
			"platform-operations",
			"trust-risk",
			"email-stack",
			"ci-cache-bootstrap",
			"security-audit-archive",
		].map(
			(scope) => `${scope}-terraform-affected=${terraform.includes(scope)}`,
		),
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
		rustRequired,
		deliveryScopes,
		databases,
		containers,
		terraform,
	}),
);
