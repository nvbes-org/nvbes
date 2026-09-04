import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";
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

const nxConfiguration = JSON.parse(readFileSync("nx.json", "utf8"));

test("keeps Nx local and disconnected from metered cloud services", () => {
	assert.equal(nxConfiguration.neverConnectToCloud, true);
	assert.equal(nxConfiguration.cacheDirectory, ".nx/cache");
	assert.equal(Object.hasOwn(nxConfiguration, "nxCloudId"), false);
});

test("only protected branch pushes share the trusted cache", () => {
	for (const ref of [
		"refs/heads/main",
		"refs/heads/dev",
		"refs/heads/release/1.0",
	]) {
		assert.equal(isTrustedPush("push", ref), true);
	}
	assert.equal(isTrustedPush("pull_request", "refs/heads/main"), false);
	assert.equal(isTrustedPush("push", "refs/heads/staging"), false);
	assert.equal(isTrustedPush("push", "refs/heads/feature/cache"), false);
});

test("rejects missing and all-zero GitHub base revisions", () => {
	assert.equal(isUsableCommitSha("a".repeat(40)), true);
	assert.equal(isUsableCommitSha("0".repeat(40)), false);
	assert.equal(isUsableCommitSha("main"), false);
});

test("versions the persistent cache by platform and Nx major", () => {
	assert.equal(
		trustedCacheDirectory({
			runnerToolCache: "/runner/tools",
			runnerOs: "macOS",
			runnerArch: "ARM64",
			nxVersion: "^23.1.1",
		}),
		"/runner/tools/nvbes/nx-cache/v1/macos-arm64/nx-23/trusted",
	);
});

test("selects affected TypeScript projects from the Nx graph", () => {
	const qualityTargets = {
		"format:check": {},
		lint: {},
		typecheck: {},
	};
	const graph = {
		core: { data: { root: "libs/rust/core" } },
		"identity-sdk-core": {
			data: { root: "libs/ts/identity-sdk-core", targets: { check: {} } },
		},
		"identity-client": {
			data: { root: "libs/ts/identity-client", targets: qualityTargets },
		},
		"web-ui": { data: { root: "libs/ts/web-ui", targets: qualityTargets } },
	};
	assert.deepEqual(
		selectTypeScriptProjects(
			["core", "identity-sdk-core", "web-ui", "identity-client"],
			graph,
		),
		["identity-sdk-core", "web-ui", "identity-client"],
	);
});

test("selects delivery scopes from owned paths without cross-runtime coupling", () => {
	const scope = {
		projects: ["account-service", "account-client"],
		paths: [
			"apps/account-service",
			"infrastructure/environments/account-production",
		],
	};
	assert.equal(isDeliveryScopeAffected(["account-client"], [], scope), false);
	assert.equal(
		isDeliveryScopeAffected([], ["apps/account-service/src/main.rs"], scope),
		true,
	);
	assert.equal(isDeliveryScopeAffected([], ["Cargo.lock"], scope), false);
	assert.equal(
		isDeliveryScopeAffected([], ["tools/ci/nx-cache-manager.mjs"], scope),
		false,
	);
	assert.equal(isDeliveryScopeAffected([], ["docs/README.md"], scope), false);
});

test("selects Rust only for Rust sources and workspace configuration", () => {
	assert.equal(isRustWorkspaceAffected(["Cargo.lock"]), true);
	assert.equal(isRustWorkspaceAffected(["libs/rust/core/src/lib.rs"]), true);
	assert.equal(
		isRustWorkspaceAffected(["apps/email-worker/Dockerfile"]),
		false,
	);
	assert.equal(isRustWorkspaceAffected(["libs/ts/web-ui/src/index.ts"]), false);
});

test("installs Rust tooling for workspace and database test scopes", () => {
	assert.equal(
		isRustToolchainRequired(true, { account: false, email: false }),
		true,
	);
	assert.equal(
		isRustToolchainRequired(false, { account: true, email: false }),
		true,
	);
	assert.equal(
		isRustToolchainRequired(false, { account: false, email: false }),
		false,
	);
});

test("selects database checks from persistence paths only", () => {
	assert.equal(
		isDatabaseScopeAffected(
			["apps/email-worker/src/email.worker.dispatch.db.rs"],
			"email",
		),
		true,
	);
	assert.equal(
		isDatabaseScopeAffected(
			["apps/email-worker/src/email.worker.dispatch.rs"],
			"email",
		),
		false,
	);
	assert.equal(
		isDatabaseScopeAffected(
			["apps/account-service/migrations/0001.sql"],
			"account",
		),
		true,
	);
	assert.equal(
		isDatabaseScopeAffected(
			["apps/billing-service/migrations/0001.sql"],
			"billing",
		),
		true,
	);
	assert.equal(
		isDatabaseScopeAffected(
			["apps/identity-service/src/identity.database.rs"],
			"identity",
		),
		true,
	);
	assert.equal(
		isDatabaseScopeAffected(
			["scripts/test-trust-risk-service-database.sh"],
			"trust-risk",
		),
		true,
	);
});

test("selects container contracts without selecting application sources", () => {
	assert.equal(
		isContainerScopeAffected(
			["apps/email-worker/Dockerfile"],
			"apps/email-worker",
		),
		true,
	);
	assert.equal(
		isContainerScopeAffected(
			["apps/email-worker/src/main.rs"],
			"apps/email-worker",
		),
		false,
	);
});

test("selects only Terraform stacks that own changed configuration", () => {
	const allTerraformScopes = [
		"account",
		"billing",
		"ci-cache-bootstrap",
		"email",
		"email-stack",
		"identity",
		"platform-operations",
		"security-audit-archive",
		"trust-risk",
	];
	assert.deepEqual(
		terraformScopes([
			"infrastructure/environments/account-production/main.tf",
			"infrastructure/stacks/email/production/main.tf",
		]),
		["account", "email-stack"],
	);
	assert.deepEqual(
		terraformScopes(["infrastructure/modules/scaleway-ci-cache/main.tf"]),
		allTerraformScopes,
	);
	assert.deepEqual(
		terraformScopes(["infrastructure/modules/terraform-state-backend/main.tf"]),
		allTerraformScopes,
	);
	assert.deepEqual(
		terraformScopes(["infrastructure/modules/scaleway-audit-archive/main.tf"]),
		allTerraformScopes,
	);
});
