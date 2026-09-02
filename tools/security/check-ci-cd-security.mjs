#!/usr/bin/env node
import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { join } from "node:path";

const registryPath = "docs/security/ci-cd-security-controls.json";
const workflowsDir = ".github/workflows";
const expectedSchemaVersion = 1;
const errors = [];
const forbiddenWorkflowTriggers = ["pull_request_target", "workflow_run"];
const forbiddenWritePermissions = [
	"actions",
	"checks",
	"contents",
	"deployments",
	"discussions",
	"id-token",
	"issues",
	"packages",
	"pages",
	"pull-requests",
	"repository-projects",
	"security-events",
	"statuses",
];
const dangerousRunPatterns = [
	/\|\|\s*true/u,
	/curl\s+[^|\n]*\|\s*(?:bash|sh)\b/u,
	/wget\s+[^|\n]*\|\s*(?:bash|sh)\b/u,
	/\bnpm\s+install\b/u,
	/\bcargo\s+install\b(?![^\n]*--locked)/u,
	/docker\s+run\b[^\n]*--privileged/u,
];
const requiredLockfileInstalls = ["pnpm install --frozen-lockfile"];
const fullCommitShaPattern = /^[0-9a-f]{40}$/u;
const githubExpression = (value) => `\${{ ${value} }}`;
const trustedCacheBranchCondition =
	"(github.ref == 'refs/heads/main' || github.ref == 'refs/heads/dev' || github.ref == 'refs/heads/staging' || startsWith(github.ref, 'refs/heads/release/'))";
const cacheEnvironmentSelector = `${trustedCacheBranchCondition} && 'production-ci-cache' || 'branch-ci-cache'`;
const cachePrefixSelector = `${trustedCacheBranchCondition} && 'trusted' || format('branches/{0}', github.ref_name)`;
const workflowScope = parseWorkflowScope(process.argv.slice(2));

function parseWorkflowScope(args) {
	if (args.length === 0) return undefined;
	if (
		args.length !== 2 ||
		args[0] !== "--workflow" ||
		!/^\.github\/workflows\/[^/]+\.ya?ml$/u.test(args[1])
	) {
		errors.push(
			"usage: check-ci-cd-security.mjs [--workflow .github/workflows/<name>.yml]",
		);
		return undefined;
	}
	return args[1];
}

function readJson(path) {
	if (!existsSync(path)) {
		errors.push(`${path}: missing`);
		return undefined;
	}

	try {
		return JSON.parse(readFileSync(path, "utf8"));
	} catch (error) {
		errors.push(`${path}: invalid JSON (${error.message})`);
		return undefined;
	}
}

function requireArray(value, path) {
	if (Array.isArray(value)) return value;
	errors.push(`${path}: must be an array`);
	return [];
}

function requireString(value, path) {
	if (typeof value === "string" && value.trim().length > 0) return value;
	errors.push(`${path}: must be a non-empty string`);
	return "";
}

function assertId(value, path, pattern) {
	const id = requireString(value, path);
	if (id && !pattern.test(id)) {
		errors.push(`${path}: invalid ID format (${id})`);
	}
	return id;
}

function requireUnique(id, seen, path) {
	if (!id) return;
	if (seen.has(id)) {
		errors.push(`${path}: duplicate ID ${id}`);
		return;
	}
	seen.add(id);
}

function requireIncludes(path, includes, context) {
	if (!existsSync(path)) {
		errors.push(`${context}.path: ${path} is missing`);
		return 0;
	}

	const text = readFileSync(path, "utf8");
	let count = 0;

	for (const include of requireArray(includes, `${context}.includes`)) {
		const needle = requireString(include, `${context}.includes[]`);
		if (!needle) continue;
		count += 1;
		if (!text.includes(needle)) {
			errors.push(
				`${context}: ${path} does not include ${JSON.stringify(needle)}`,
			);
		}
	}

	return count;
}

function workflowFiles() {
	if (!existsSync(workflowsDir)) return [];
	return readdirSync(workflowsDir)
		.map((name) => join(workflowsDir, name))
		.filter((path) => statSync(path).isFile() && /\.(ya?ml)$/u.test(path));
}

function extractUses(text) {
	return [...text.matchAll(/^\s*uses:\s*["']?([^"'\s#]+)["']?/gmu)].map(
		(match) => match[1],
	);
}

function extractSecrets(text) {
	return [...text.matchAll(/secrets\.([A-Z0-9_]+)/gu)].map((match) => match[1]);
}

function lineNumberAt(text, index) {
	return text.slice(0, index).split("\n").length;
}

function assertWorkflowBaseline(path, text, registry) {
	for (const include of requireArray(
		registry.requiredWorkflowIncludes,
		"requiredWorkflowIncludes",
	)) {
		if (!text.includes(include)) {
			errors.push(
				`${path}: required workflow baseline missing ${JSON.stringify(include)}`,
			);
		}
	}

	if (!text.includes("node tools/security/check-ci-cd-security.mjs")) {
		errors.push(
			`${path}: CI/CD security gate must run before dependency installation`,
		);
	}

	const securityGateIndex = text.indexOf(
		"node tools/security/check-ci-cd-security.mjs",
	);
	const installIndex = text.indexOf("pnpm install --frozen-lockfile");
	if (
		securityGateIndex >= 0 &&
		installIndex >= 0 &&
		securityGateIndex > installIndex
	) {
		errors.push(
			`${path}: CI/CD security gate must run before dependency installation`,
		);
	}
}

function assertForbiddenTriggers(path, text) {
	for (const trigger of forbiddenWorkflowTriggers) {
		if (new RegExp(`^\\s*${trigger}\\s*:`, "mu").test(text)) {
			errors.push(`${path}: dangerous trigger ${trigger} is forbidden`);
		}
	}
}

function assertPermissions(path, text, allowedWritePermissions) {
	if (!/^permissions:\s*$/mu.test(text)) {
		errors.push(`${path}: top-level permissions: block is required`);
	}

	for (const permission of forbiddenWritePermissions) {
		const pattern = new RegExp(`^\\s*${permission}:\\s*write\\s*$`, "mu");
		if (pattern.test(text) && !allowedWritePermissions.includes(permission)) {
			errors.push(
				`${path}: write permission ${permission}: write is not allowlisted`,
			);
		}
	}
}

function assertActions(path, text, allowedActions) {
	for (const action of extractUses(text)) {
		if (action.startsWith("./")) continue;
		if (!allowedActions.includes(action)) {
			errors.push(`${path}: action ${action} is not allowlisted`);
		}
		const separator = action.lastIndexOf("@");
		const ref = separator >= 0 ? action.slice(separator + 1) : "";
		if (!fullCommitShaPattern.test(ref)) {
			errors.push(
				`${path}: action ${action} must be pinned to a full commit SHA`,
			);
		}
	}
}

function assertSecrets(path, text, allowedSecrets) {
	const referencedSecrets = extractSecrets(text);
	const protectedSecretWorkflows = new Map([
		[
			"ACCOUNT_ACCEPTANCE_TRUSTED_PUBLIC_KEY_PEM",
			new Set([
				".github/workflows/account-acceptance-ingest.yml",
				".github/workflows/account-release.yml",
			]),
		],
		[
			"ACCOUNT_DEPLOYMENT_TRUSTED_PUBLIC_KEY_PEM",
			new Set([
				".github/workflows/account-acceptance-ingest.yml",
				".github/workflows/account-release.yml",
			]),
		],
		[
			"NVBES_STAGING_ACCOUNT_PASSWORD",
			new Set([".github/workflows/account-release.yml"]),
		],
		...[
			"IDENTITY_MFA_ENCRYPTION_KEY",
			"IDENTITY_METRICS_TOKEN",
			"IDENTITY_SENTRY_DSN",
			"IDENTITY_SYNTHETIC_PASSWORD",
			"IDENTITY_SYNTHETIC_RECOVERED_PASSWORD",
		].map((secret) => [
			secret,
			new Set([".github/workflows/deploy-identity.yml"]),
		]),
		...[
			"IDENTITY_TERRAFORM_STATE_ACCESS_KEY",
			"IDENTITY_TERRAFORM_STATE_SECRET_KEY",
		].map((secret) => [
			secret,
			new Set([
				".github/workflows/deploy-identity.yml",
				".github/workflows/validate-identity-restore.yml",
			]),
		]),
	]);
	for (const secret of referencedSecrets) {
		if (!allowedSecrets.includes(secret)) {
			errors.push(`${path}: secret ${secret} is not allowlisted`);
		}
		const allowedWorkflows = protectedSecretWorkflows.get(secret);
		if (allowedWorkflows && !allowedWorkflows.has(path)) {
			errors.push(
				`${path}: protected release secret ${secret} is not allowed here`,
			);
		}
	}

	const secretIndex = text.indexOf("secrets.");
	if (secretIndex >= 0) {
		const preceding = text.slice(Math.max(0, secretIndex - 500), secretIndex);
		const isValidatedDastWorkflow =
			path === ".github/workflows/dast.yml" &&
			/^\s+schedule:\s*$/mu.test(text) &&
			/^\s+workflow_dispatch:\s*$/mu.test(text) &&
			text.includes("name: account-dast-staging") &&
			text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
			text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
			text.includes(`ref: ${githubExpression("github.sha")}`) &&
			text.includes("node tools/security/validate-dast-targets.mjs") &&
			text.includes("DAST_ALLOWED_ORIGINS");
		const isValidatedAccountReleaseWorkflow =
			path === ".github/workflows/account-release.yml" &&
			/^\s+workflow_dispatch:\s*$/mu.test(text) &&
			!text.includes("inputs.") &&
			text.includes("name: production-account") &&
			text.includes("verify-account-release-ref.mjs") &&
			text.includes(`ref: ${githubExpression("github.ref")}`) &&
			text.includes(`NVBES_RELEASE_SHA: ${githubExpression("github.sha")}`) &&
			text.includes("RELEASE_APPROVED: production") &&
			text.includes("scripts/release-gate.sh production");
		const isValidatedAcceptanceIngestWorkflow =
			path === ".github/workflows/account-acceptance-ingest.yml" &&
			/^\s+workflow_dispatch:\s*$/mu.test(text) &&
			!text.includes("inputs.") &&
			text.includes("name: account-acceptance") &&
			text.includes("verify-account-release-ref.mjs") &&
			text.includes(`ref: ${githubExpression("github.ref")}`) &&
			text.includes(`NVBES_RELEASE_SHA: ${githubExpression("github.sha")}`) &&
			text.includes(
				'run.path !== ".github/workflows/account-acceptance-source.yml"',
			) &&
			text.includes(
				"node tools/account-quality/verify-acceptance-evidence.mjs",
			) &&
			text.includes("prepare-account-acceptance-publication.mjs") &&
			text.includes(
				`name: account-acceptance-${githubExpression("github.sha")}`,
			);
		const isValidatedEmailDeploymentWorkflow =
			path === ".github/workflows/deploy-email.yml" &&
			/^\s+workflow_dispatch:\s*$/mu.test(text) &&
			!/^\s+inputs:\s*$/mu.test(text) &&
			text.includes("name: production-email") &&
			text.includes("if: github.ref == 'refs/heads/main'") &&
			text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
			text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
			text.includes(
				`EMAIL_DEPLOY_CONFIRMATION: ${githubExpression("vars.EMAIL_DEPLOY_CONFIRMATION")}`,
			) &&
			text.includes(
				`EMAIL_DEPLOY_APPROVED_SHA: ${githubExpression("vars.EMAIL_DEPLOY_APPROVED_SHA")}`,
			) &&
			text.includes(
				'[[ "$EMAIL_DEPLOY_CONFIRMATION" == "deploy-email-production" ]]',
			) &&
			text.includes('[[ "$EMAIL_DEPLOY_APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]') &&
			text.includes('[[ "$EMAIL_DEPLOY_APPROVED_SHA" == "$GITHUB_SHA" ]]') &&
			text.includes(`ref: ${githubExpression("github.sha")}`) &&
			text.includes("production/email/terraform.tfstate") &&
			text.includes("ghcr.io/nvbes-org/nvbes-email-worker") &&
			text.includes("cosign verify") &&
			text.includes("email-runtime.tfplan");
		const isValidatedEmailRestoreWorkflow =
			path === ".github/workflows/validate-email-restore.yml" &&
			/^\s+workflow_dispatch:\s*$/mu.test(text) &&
			text.includes("name: production-email") &&
			text.includes("if: github.ref == 'refs/heads/main'") &&
			text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
			text.includes(
				'[[ "$CONFIRMATION" == "validate-email-production-restore" ]]',
			) &&
			text.includes('[[ "$APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]') &&
			text.includes('[[ "$APPROVED_SHA" == "$(git rev-parse HEAD)" ]]') &&
			text.includes(`ref: ${githubExpression("inputs.approved_sha")}`) &&
			text.includes("production/email/terraform.tfstate") &&
			text.includes("restore_database_id");
		const isValidatedTrustRiskDeploymentWorkflow =
			path === ".github/workflows/deploy-trust-risk.yml" &&
			/^\s+workflow_dispatch:\s*$/mu.test(text) &&
			!/^\s+inputs:\s*$/mu.test(text) &&
			text.includes("name: production-trust-risk") &&
			text.includes("if: github.ref == 'refs/heads/main'") &&
			text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
			text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
			text.includes(`ref: ${githubExpression("github.sha")}`) &&
			text.includes("production/trust-risk/terraform.tfstate") &&
			text.includes("ghcr.io/nvbes-org/nvbes-trust-risk-service") &&
			text.includes("cosign verify") &&
			text.includes("trust-risk-runtime.tfplan");
		const isValidatedIdentityDeploymentWorkflow =
			path === ".github/workflows/deploy-identity.yml" &&
			/^\s+workflow_dispatch:\s*$/mu.test(text) &&
			!/^\s+inputs:\s*$/mu.test(text) &&
			text.includes("name: production-identity") &&
			text.includes("if: github.ref == 'refs/heads/main'") &&
			text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
			text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
			text.includes(
				`IDENTITY_DEPLOY_CONFIRMATION: ${githubExpression("vars.IDENTITY_DEPLOY_CONFIRMATION")}`,
			) &&
			text.includes(
				`IDENTITY_DEPLOY_APPROVED_SHA: ${githubExpression("vars.IDENTITY_DEPLOY_APPROVED_SHA")}`,
			) &&
			text.includes(
				'[[ "$IDENTITY_DEPLOY_CONFIRMATION" == "deploy-identity-production" ]]',
			) &&
			text.includes(
				'[[ "$IDENTITY_DEPLOY_APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]',
			) &&
			text.includes('[[ "$IDENTITY_DEPLOY_APPROVED_SHA" == "$GITHUB_SHA" ]]') &&
			text.includes(`ref: ${githubExpression("github.sha")}`) &&
			text.includes("production/identity/terraform.tfstate") &&
			text.includes("ghcr.io/nvbes-org/nvbes-identity-service") &&
			text.includes("cosign verify") &&
			text.includes("identity-runtime.tfplan");
		const isValidatedIdentityRestoreWorkflow =
			path === ".github/workflows/validate-identity-restore.yml" &&
			/^\s+workflow_dispatch:\s*$/mu.test(text) &&
			text.includes("name: production-identity") &&
			text.includes("if: github.ref == 'refs/heads/main'") &&
			text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
			text.includes(
				'[[ "$CONFIRMATION" == "validate-identity-production-restore" ]]',
			) &&
			text.includes('[[ "$APPROVED_SHA" =~ ^[0-9a-f]{40}$ ]]') &&
			text.includes('[[ "$APPROVED_SHA" == "$(git rev-parse HEAD)" ]]') &&
			text.includes(`ref: ${githubExpression("inputs.approved_sha")}`) &&
			text.includes("production/identity/terraform.tfstate") &&
			text.includes("expected_principal_id") &&
			text.includes("DELETE_RESTORE_DATABASE=true");
		const isValidatedCiCacheRotationWorkflow =
			path === ".github/workflows/rotate-ci-cache-credentials.yml" &&
			/^\s+schedule:\s*$/mu.test(text) &&
			/^\s+workflow_dispatch:\s*$/mu.test(text) &&
			text.includes("name: production-bootstrap") &&
			text.includes("if: github.ref == 'refs/heads/main'") &&
			text.includes('[[ "$GITHUB_REF" == "refs/heads/main" ]]') &&
			text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
			text.includes(`ref: ${githubExpression("github.sha")}`) &&
			text.includes("production/bootstrap/terraform.tfstate") &&
			text.includes("repair_bucket_policy:") &&
			text.includes("if: inputs.repair_bucket_policy == true") &&
			text.includes("-refresh=false") &&
			text.includes(
				"-target=module.ci_cache.scaleway_object_bucket_policy.ci_cache",
			) &&
			text.includes("ci-cache-policy-repair.tfplan") &&
			text.includes("-target=module.ci_cache") &&
			text.includes("ci-cache-rotation.tfplan");
		const isValidatedBranchCacheWorkflow =
			path === ".github/workflows/ci.yml" &&
			/^ {2}push:\n {4}branches: \['\*\*'\]$/mu.test(text) &&
			text.includes(
				"rust-tests-scaleway-cache:\n    if: github.event_name == 'push'",
			) &&
			text.includes(
				`environment:\n      name: ${githubExpression(cacheEnvironmentSelector)}\n      deployment: false`,
			) &&
			text.includes("if: github.event_name == 'push'") &&
			text.includes('[[ "$GITHUB_EVENT_NAME" == "push" ]]') &&
			text.includes('[[ "$GITHUB_REF" == refs/heads/* ]]') &&
			text.includes('[[ "$(git rev-parse HEAD)" == "$GITHUB_SHA" ]]') &&
			text.includes(
				`SCCACHE_S3_KEY_PREFIX: ${githubExpression(cachePrefixSelector)}/rust/${githubExpression("runner.os")}-${githubExpression("runner.arch")}/rust-1.91.1`,
			) &&
			text.includes(
				`AWS_ACCESS_KEY_ID: ${githubExpression("secrets.SCW_CI_CACHE_ACCESS_KEY")}`,
			) &&
			text.includes(
				`AWS_SECRET_ACCESS_KEY: ${githubExpression("secrets.SCW_CI_CACHE_SECRET_KEY")}`,
			);
		if (
			!preceding.includes(
				"if: github.event_name == 'push' && github.ref == 'refs/heads/main'",
			) &&
			!isValidatedDastWorkflow &&
			!isValidatedAccountReleaseWorkflow &&
			!isValidatedAcceptanceIngestWorkflow &&
			!isValidatedEmailDeploymentWorkflow &&
			!isValidatedEmailRestoreWorkflow &&
			!isValidatedTrustRiskDeploymentWorkflow &&
			!isValidatedIdentityDeploymentWorkflow &&
			!isValidatedIdentityRestoreWorkflow &&
			!isValidatedCiCacheRotationWorkflow &&
			!isValidatedBranchCacheWorkflow
		) {
			errors.push(
				`${path}: secret-bearing step must be restricted to trusted push on main or a validated protected-Environment workflow`,
			);
		}
	}
}

function assertRunSafety(path, text) {
	for (const pattern of dangerousRunPatterns) {
		const match = pattern.exec(text);
		if (match) {
			errors.push(
				`${path}:${lineNumberAt(text, match.index)}: dangerous run pattern ${pattern}`,
			);
		}
	}

	if (/\bpnpm\s+(?:install|add|update|exec|run)\b/u.test(text)) {
		for (const install of requiredLockfileInstalls) {
			if (!text.includes(install)) {
				errors.push(
					`${path}: unsafe dependency install policy, expected ${install}`,
				);
			}
		}
	}
}

function assertContinueOnError(
	path,
	text,
	allowedContinueOnError,
	usedAllowances,
) {
	const lines = text.split("\n");
	for (const [index, line] of lines.entries()) {
		const match = /^(\s*)continue-on-error\s*:/u.exec(line);
		if (!match) continue;

		const fieldIndent = match[1].length;
		const stepIndent = fieldIndent - 2;
		let stepStart = -1;
		for (let cursor = index - 1; cursor >= 0; cursor -= 1) {
			const candidate = lines[cursor];
			if (candidate.trim().length === 0) continue;
			const candidateIndent = /^\s*/u.exec(candidate)?.[0].length ?? 0;
			if (
				candidateIndent === stepIndent &&
				/^\s*-\s+(?:name|id|uses|run)\s*:/u.test(candidate)
			) {
				stepStart = cursor;
				break;
			}
			if (candidateIndent < stepIndent) break;
		}

		let stepId;
		if (stepStart >= 0) {
			const idPattern = new RegExp(
				`^\\s{${fieldIndent}}id\\s*:\\s*([A-Za-z0-9_-]+)\\s*$`,
				"u",
			);
			for (let cursor = stepStart; cursor <= index; cursor += 1) {
				const idMatch = idPattern.exec(lines[cursor]);
				if (idMatch) {
					stepId = idMatch[1];
					break;
				}
			}
		}

		const allowance = stepId ? `${path}#${stepId}` : undefined;
		if (allowance && allowedContinueOnError.has(allowance)) {
			usedAllowances.add(allowance);
			continue;
		}
		errors.push(
			`${path}:${index + 1}: continue-on-error is forbidden without an exact workflow#step-id allowlist entry`,
		);
	}
}

function assertCheckout(path, text) {
	if (
		text.includes("actions/checkout") &&
		!text.includes("persist-credentials: false")
	) {
		errors.push(
			`${path}: actions/checkout must use persist-credentials: false`,
		);
	}
}

const registry = readJson(registryPath);

if (registry) {
	if (registry.schemaVersion !== expectedSchemaVersion) {
		errors.push(
			`${registryPath}: schemaVersion must be ${expectedSchemaVersion}`,
		);
	}

	requireString(registry.source, `${registryPath}.source`);
	requireString(registry.reviewCadence, `${registryPath}.reviewCadence`);

	const allowedActions = requireArray(
		registry.allowedActions,
		`${registryPath}.allowedActions`,
	);
	const allowedSecrets = requireArray(
		registry.allowedSecrets,
		`${registryPath}.allowedSecrets`,
	);
	const allowedWritePermissions = requireArray(
		registry.allowedWritePermissions,
		`${registryPath}.allowedWritePermissions`,
	);
	const allowedContinueOnError = requireArray(
		registry.allowedContinueOnError,
		`${registryPath}.allowedContinueOnError`,
	);
	const allowedContinueOnErrorSet = new Set();
	for (const [index, allowance] of allowedContinueOnError.entries()) {
		const value = requireString(
			allowance,
			`${registryPath}.allowedContinueOnError[${index}]`,
		);
		if (
			value &&
			!/^\.github\/workflows\/[^#\s]+\.ya?ml#[A-Za-z0-9_-]+$/u.test(value)
		) {
			errors.push(
				`${registryPath}.allowedContinueOnError[${index}]: must be an exact .github/workflows/*.yml#step-id entry`,
			);
		}
		requireUnique(
			value,
			allowedContinueOnErrorSet,
			`${registryPath}.allowedContinueOnError`,
		);
	}
	const usedContinueOnErrorAllowances = new Set();

	const requirements = requireArray(
		registry.requirements,
		`${registryPath}.requirements`,
	);
	const requirementIds = new Set();
	const controlIds = new Set();
	let controlCount = 0;
	let evidenceCount = 0;

	requirements.forEach((requirement, requirementIndex) => {
		const requirementContext = `requirements[${requirementIndex}]`;
		const requirementId = assertId(
			requirement?.id,
			`${requirementContext}.id`,
			/^CICD_REQ_\d{3}$/u,
		);
		requireUnique(requirementId, requirementIds, "requirements");
		requireString(requirement?.name, `${requirementContext}.name`);
		requireString(requirement?.owasp, `${requirementContext}.owasp`);

		const controls = requireArray(
			requirement?.controls,
			`${requirementContext}.controls`,
		);
		if (controls.length === 0) {
			errors.push(`${requirementContext}: must include at least one control`);
		}

		controls.forEach((control, controlIndex) => {
			const controlContext = `${requirementContext}.controls[${controlIndex}]`;
			const controlId = assertId(
				control?.id,
				`${controlContext}.id`,
				/^CICD_CTRL_\d{3}$/u,
			);
			requireUnique(controlId, controlIds, "controls");
			requireString(control?.name, `${controlContext}.name`);
			requireString(control?.description, `${controlContext}.description`);
			controlCount += 1;

			const evidence = requireArray(
				control?.evidence,
				`${controlContext}.evidence`,
			);
			if (evidence.length === 0) {
				errors.push(
					`${controlContext}: must include at least one evidence entry`,
				);
			}

			evidence.forEach((entry, evidenceIndex) => {
				const evidenceContext = `${controlContext}.evidence[${evidenceIndex}]`;
				const path = requireString(entry?.path, `${evidenceContext}.path`);
				if (workflowScope) {
					for (const include of requireArray(
						entry?.includes,
						`${evidenceContext}.includes`,
					)) {
						requireString(include, `${evidenceContext}.includes[]`);
						evidenceCount += 1;
					}
				} else {
					evidenceCount += requireIncludes(
						path,
						entry?.includes,
						evidenceContext,
					);
				}
			});
		});
	});

	const availableWorkflows = workflowFiles();
	const scopedWorkflowExists =
		!workflowScope || availableWorkflows.includes(workflowScope);
	const workflows = workflowScope
		? scopedWorkflowExists
			? [workflowScope]
			: []
		: availableWorkflows;
	if (workflows.length === 0) {
		errors.push(`${workflowsDir}: no workflow files found`);
	}
	if (workflowScope && !scopedWorkflowExists) {
		errors.push(`${workflowScope}: workflow does not exist`);
	}

	for (const path of workflows) {
		const text = readFileSync(path, "utf8");
		assertWorkflowBaseline(path, text, registry);
		assertForbiddenTriggers(path, text);
		assertPermissions(path, text, allowedWritePermissions);
		assertActions(path, text, allowedActions);
		assertSecrets(path, text, allowedSecrets);
		assertRunSafety(path, text);
		assertContinueOnError(
			path,
			text,
			allowedContinueOnErrorSet,
			usedContinueOnErrorAllowances,
		);
		assertCheckout(path, text);
	}

	for (const allowance of allowedContinueOnErrorSet) {
		if (!usedContinueOnErrorAllowances.has(allowance)) {
			errors.push(
				`${registryPath}.allowedContinueOnError: unused allowance ${allowance}`,
			);
		}
	}

	if (errors.length === 0) {
		console.log(
			`CI/CD security controls: ok (${requirements.length} requirements, ${controlCount} controls, ${workflows.length} workflows, ${evidenceCount} evidence strings)`,
		);
	}
}

if (errors.length > 0) {
	console.error("CI/CD security controls failed:");
	for (const error of errors) {
		console.error(`- ${error}`);
	}
	process.exit(1);
}
