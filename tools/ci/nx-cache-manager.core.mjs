import { join } from "node:path";

const trustedRefs = new Set(["refs/heads/main", "refs/heads/dev"]);

const rustWorkspacePaths = new Set([
	"Cargo.lock",
	"Cargo.toml",
	"rust-toolchain.toml",
	"rustfmt.toml",
]);

const typeScriptQualityTargets = [
	"format:check",
	"lint",
	"typecheck",
	"check",
	"test",
];

export function isTrustedPush(eventName, ref) {
	return (
		eventName === "push" &&
		(trustedRefs.has(ref) || ref.startsWith("refs/heads/release/"))
	);
}

export function isUsableCommitSha(value) {
	return /^[0-9a-f]{40}$/u.test(value) && !/^0{40}$/u.test(value);
}

export function nxMajor(version) {
	const match = version.match(/(\d+)\./u);
	if (match === null) throw new Error(`Unsupported Nx version: ${version}`);
	return match[1];
}

function safeSegment(value) {
	const segment = value.toLowerCase().replaceAll(/[^a-z0-9_-]/gu, "-");
	if (segment.length === 0)
		throw new Error("Cache path segment cannot be empty");
	return segment;
}

export function trustedCacheDirectory({
	runnerToolCache,
	runnerOs,
	runnerArch,
	nxVersion,
}) {
	return join(
		runnerToolCache,
		"nvbes",
		"nx-cache",
		"v1",
		`${safeSegment(runnerOs)}-${safeSegment(runnerArch)}`,
		`nx-${nxMajor(nxVersion)}`,
		"trusted",
	);
}

export function selectTypeScriptProjects(affectedProjects, graphNodes) {
	return affectedProjects.filter((project) => {
		const node = graphNodes[project];
		const targets = node?.data?.targets;
		return (
			node?.data?.root?.startsWith("libs/ts/") === true &&
			targets !== undefined &&
			typeScriptQualityTargets.some((target) => Object.hasOwn(targets, target))
		);
	});
}

function matchesPath(path, prefixes) {
	return prefixes.some(
		(prefix) =>
			path === prefix ||
			path.startsWith(`${prefix}/`) ||
			path.startsWith(`${prefix}.`),
	);
}

export function isRustWorkspaceAffected(changedPaths) {
	return changedPaths.some(
		(path) =>
			rustWorkspacePaths.has(path) ||
			path.startsWith(".cargo/") ||
			path.startsWith("vendor/xmlsec/") ||
			((path.startsWith("apps/") || path.startsWith("libs/rust/")) &&
				(path.endsWith(".rs") || path.endsWith("Cargo.toml"))) ||
			path.startsWith("contracts/protobuf/"),
	);
}

export function isRustToolchainRequired(rustAffected, databases) {
	return rustAffected || Object.values(databases).some(Boolean);
}

export function isDatabaseScopeAffected(changedPaths, scope) {
	const patterns = {
		account: [
			"apps/account-service/migrations",
			"apps/account-service/src/account.db",
			"apps/account-service/src/account.database",
			"scripts/test-account-service-database.sh",
			"scripts/validate-account-test-database",
		],
		billing: [
			"apps/billing-service/migrations",
			"apps/billing-service/src/billing.db",
			"apps/billing-service/src/billing.database",
			"scripts/test-billing-service-database.sh",
			"scripts/validate-billing-test-database",
		],
		email: [
			"apps/email-worker/migrations",
			"apps/email-worker/src/email.worker.database",
			"apps/email-worker/src/email.worker.dispatch.db",
			"apps/email-worker/src/email.worker.metrics.db",
			"apps/email-worker/src/email.worker.webhook.db",
			"scripts/test-email-worker-database.sh",
			"scripts/validate-email-test-database",
		],
		identity: [
			"apps/identity-service/migrations",
			"apps/identity-service/src/identity.db",
			"apps/identity-service/src/identity.database",
			"scripts/test-identity-service-database.sh",
			"scripts/validate-identity-test-database",
		],
		"trust-risk": [
			"apps/trust-risk-service/migrations",
			"apps/trust-risk-service/src/trust_risk.db",
			"apps/trust-risk-service/src/trust_risk.database",
			"scripts/test-trust-risk-service-database.sh",
			"scripts/validate-trust-risk-test-database",
		],
	};
	return changedPaths.some((path) => matchesPath(path, patterns[scope] ?? []));
}

export function isContainerScopeAffected(changedPaths, projectRoot) {
	return changedPaths.some(
		(path) =>
			path === `${projectRoot}/Dockerfile` ||
			path.startsWith(`${projectRoot}/tests/container-contract.`),
	);
}

export function terraformScopes(changedPaths) {
	const scopes = new Set();
	const environments = [
		"account",
		"billing",
		"email",
		"identity",
		"platform-operations",
		"trust-risk",
	];
	for (const scope of environments) {
		if (
			changedPaths.some((path) =>
				path.startsWith(`infrastructure/environments/${scope}-production/`),
			)
		) {
			scopes.add(scope);
		}
	}
	if (changedPaths.some((path) => path.startsWith("infrastructure/modules/"))) {
		for (const scope of environments) scopes.add(scope);
		scopes.add("email-stack");
		scopes.add("ci-cache-bootstrap");
		scopes.add("security-audit-archive");
	}
	if (
		changedPaths.some((path) => path.startsWith("infrastructure/stacks/email/"))
	) {
		scopes.add("email-stack");
	}
	if (
		changedPaths.some((path) =>
			path.startsWith("infrastructure/bootstrap/production/initial/"),
		)
	) {
		scopes.add("ci-cache-bootstrap");
	}
	if (
		changedPaths.some((path) =>
			path.startsWith("infrastructure/modules/scaleway-v1/"),
		)
	) {
		for (const scope of environments) scopes.add(scope);
	}
	if (
		changedPaths.some((path) =>
			path.startsWith("infrastructure/modules/scaleway-ci-cache/"),
		)
	) {
		scopes.add("ci-cache-bootstrap");
	}
	if (
		changedPaths.some((path) =>
			path.startsWith("infrastructure/modules/terraform-state-backend/"),
		)
	) {
		scopes.add("ci-cache-bootstrap");
	}
	if (
		changedPaths.some((path) =>
			path.startsWith("infrastructure/modules/scaleway-audit-archive/"),
		)
	) {
		scopes.add("security-audit-archive");
	}
	if (
		changedPaths.some((path) =>
			path.startsWith("infrastructure/environments/security-audit-archive/"),
		)
	) {
		scopes.add("security-audit-archive");
	}
	return [...scopes].sort();
}

export function isDeliveryScopeAffected(
	affectedProjects,
	changedPaths,
	{ projects, paths },
) {
	void affectedProjects;
	void projects;
	return changedPaths.some((path) => matchesPath(path, paths));
}
