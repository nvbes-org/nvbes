#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";
import { checkBillingMigrationBoundaries } from "./check-product-boundaries.billing-migrations.mjs";
import { checkBillingWorkerQueueBoundaries } from "./check-product-boundaries.billing-worker-queues.mjs";
import { checkIdentityBillingRuntimeBoundary } from "./check-product-boundaries.identity-billing-runtime.mjs";
import { checkIdentityWebBillingClientBoundary } from "./check-product-boundaries.identity-web-billing.mjs";

const errors = [];
const textExtensions = new Set([".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".sql"]);
const skippedDirs = new Set([".git", ".nx", "coverage", "dist", "node_modules", "target"]);

function normalizePath(value) {
	return value.replaceAll("\\", "/");
}

function run(command, args) {
	return execFileSync(command, args, {
		encoding: "utf8",
		stdio: ["ignore", "pipe", "pipe"],
	});
}

function isProductPath(path) {
	const normalized = normalizePath(path);
	return normalized.startsWith("libs/rust/products/") || normalized.startsWith("libs/ts/products/");
}

function scopeForPath(path) {
	const normalized = normalizePath(path);
	if (
		normalized.startsWith("apps/internal-") ||
		normalized.startsWith("libs/rust/internal/") ||
		normalized.startsWith("libs/ts/internal-") ||
		normalized.includes("/internal/")
	) {
		return "internal";
	}
	if (
		normalized.startsWith("apps/cloud-") ||
		normalized.startsWith("libs/rust/cloud/") ||
		normalized.startsWith("libs/rust/adapters-cloud/") ||
		normalized.startsWith("libs/ts/cloud-ui/")
	) {
		return "cloud";
	}
	if (normalized.startsWith("libs/rust/adapters-oss/")) {
		return "adapter";
	}
	if (normalized.startsWith("apps/")) {
		return "app";
	}
	return "oss";
}

function checkRustProductPackages() {
	const metadata = JSON.parse(run("cargo", ["metadata", "--format-version", "1", "--no-deps"]));
	const workspaceIds = new Set(metadata.workspace_members);
	const workspacePackages = metadata.packages.filter((pkg) => workspaceIds.has(pkg.id));
	const packagesByName = new Map(workspacePackages.map((pkg) => [pkg.name, pkg]));
	const directDeps = new Map();

	for (const pkg of workspacePackages) {
		directDeps.set(
			pkg.name,
			pkg.dependencies
				.map((dep) => packagesByName.get(dep.name))
				.filter(Boolean)
				.map((depPkg) => depPkg.name),
		);
	}

	function transitiveDeps(pkgName, seen = new Set()) {
		for (const depName of directDeps.get(pkgName) ?? []) {
			if (seen.has(depName)) continue;
			seen.add(depName);
			transitiveDeps(depName, seen);
		}
		return seen;
	}

	for (const pkg of workspacePackages) {
		const manifestPath = normalizePath(relative(process.cwd(), pkg.manifest_path));
		if (!isProductPath(manifestPath)) continue;

		for (const depName of transitiveDeps(pkg.name)) {
			const depPkg = packagesByName.get(depName);
			if (!depPkg) continue;
			const depPath = normalizePath(relative(process.cwd(), depPkg.manifest_path));
			const depScope = scopeForPath(depPath);
			if (["adapter", "cloud", "internal", "app"].includes(depScope)) {
				errors.push(`${pkg.name} cannot depend on ${depScope} package ${depName}`);
			}
		}
	}
}

function shouldScan(path) {
	if (path.split("/").some((part) => skippedDirs.has(part))) return false;
	const dot = path.lastIndexOf(".");
	return dot >= 0 && textExtensions.has(path.slice(dot));
}

function walk(dir, results = []) {
	if (!existsSync(dir)) return results;
	for (const entry of readdirSync(dir)) {
		const path = join(dir, entry);
		const stat = statSync(path);
		if (stat.isDirectory()) {
			walk(path, results);
		} else {
			const relativePath = normalizePath(relative(process.cwd(), path));
			if (shouldScan(relativePath)) results.push(relativePath);
		}
	}
	return results;
}

function checkProductSourceImports() {
	for (const root of ["libs/rust/products", "libs/ts/products"]) {
		for (const file of walk(root)) {
			const content = readFileSync(file, "utf8");
			const forbiddenPatterns = [
				/libs\/rust\/adapters-(?:oss|cloud)\//,
				/adapters[-_](?:oss|cloud)/,
				/libs\/ts\/cloud-ui/,
				/from\s+["'][^"']*cloud-ui["']/,
				/from\s+["'][^"']*apps\//,
			];

			for (const pattern of forbiddenPatterns) {
				if (pattern.test(content)) {
					errors.push(`${file}: product source imports forbidden boundary ${pattern}`);
				}
			}
		}
	}
}

function checkDriveBillingBoundary() {
	const forbiddenDriveBillingFiles = [
		/drive\.domains\.billing\.manage(?:\.|$)/,
		/drive\.domains\.billing\.webhooks(?:\.|$)/,
		/drive\.domains\.billing\.routes\.manage\.rs$/,
		/drive\.domains\.billing\.routes\.webhooks\.rs$/,
		/drive\.domains\.billing\.service\.rs$/,
		/drive\.domains\.billing\.stripe\.rs$/,
		/drive\.domains\.billing\.types\.rs$/,
		/drive\.domains\.billing\.usage_events\.rs$/,
	];

	for (const file of walk("apps/drive-api/src")) {
		const content = readFileSync(file, "utf8");
		for (const pattern of forbiddenDriveBillingFiles) {
			if (pattern.test(file)) {
				errors.push(`${file}: Drive API must not own Billing runtime, PSP, or public Billing route files`);
			}
		}
		if (content.includes("nvbes_billing")) {
			errors.push(`${file}: Drive API must not import nvbes_billing; use Billing service boundaries instead`);
		}
		if (/["']\/workspaces\/[^"']*\/billing(?:\/|\{|["'])/.test(content)) {
			errors.push(`${file}: Drive API must not expose workspace Billing routes`);
		}
		if (/["']\/billing\/webhooks["']/.test(content)) {
			errors.push(`${file}: Drive API must not expose PSP Billing webhook routes`);
		}
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+(?:subscriptions|billing_accounts|billing_adjustments|billing_webhook_events|invoice_estimates|usage_snapshots|stripe_price_mappings|billing_provider_customers|billing_provider_price_mappings)\b/i.test(content)) {
			errors.push(`${file}: Drive API runtime must not access Billing-owned SQL tables`);
		}
	}

	for (const file of walk("apps/drive-worker/src")) {
		const content = readFileSync(file, "utf8");
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+(?:subscriptions|billing_accounts|billing_adjustments|billing_webhook_events|invoice_estimates|usage_snapshots|stripe_price_mappings|billing_provider_customers|billing_provider_price_mappings)\b/i.test(content)) {
			errors.push(`${file}: Drive worker runtime must not access Billing-owned SQL tables`);
		}
	}

	const driveManifest = "apps/drive-api/Cargo.toml";
	if (existsSync(driveManifest) && readFileSync(driveManifest, "utf8").includes("nvbes-billing")) {
		errors.push(`${driveManifest}: Drive API must not depend on nvbes-billing`);
	}

	const driveOpenapi = "apps/drive-api/openapi.json";
	if (existsSync(driveOpenapi)) {
		const content = readFileSync(driveOpenapi, "utf8");
		if (/["']\/[^"']*\/billing(?:\/|\{|["'])/.test(content)) {
			errors.push(`${driveOpenapi}: Drive OpenAPI must not expose Billing routes`);
		}
		if (/"name"\s*:\s*"billing"/i.test(content) || /"tags"\s*:\s*\[[^\]]*"billing"/i.test(content)) {
			errors.push(`${driveOpenapi}: Drive OpenAPI must not expose Billing tags`);
		}
	}
}

function checkDriveBillingMigrationBoundary() {
	const allowedProjectionPatterns = [
		/\bbilling_entitlement_snapshots\b/i,
		/\bbilling_locked\b/i,
	];
	const forbiddenMigrationPatterns = [
		/\bbilling_provider\b/i,
		/\bsubscription_status\b/i,
		/\bcustomer_type\b/i,
		/\bbilling_adjustment_type\b/i,
		/\bbilling_webhook_status\b/i,
		/\bsubscriptions\b/i,
		/\bbilling_accounts\b/i,
		/\bbilling_adjustments\b/i,
		/\bbilling_webhook_events\b/i,
		/\binvoice_estimates\b/i,
		/\busage_snapshots\b/i,
		/\bstripe_price_mappings\b/i,
		/\bbilling_provider_customers\b/i,
		/\bbilling_provider_price_mappings\b/i,
		/\bbilling_fraud_assessments\b/i,
	];

	for (const file of walk("apps/drive-api/migrations")) {
		const content = readFileSync(file, "utf8");
		for (const pattern of forbiddenMigrationPatterns) {
			if (pattern.test(content)) {
				errors.push(`${file}: Drive migrations must not own Billing runtime schema ${pattern}`);
			}
		}
		if (/\bbilling_[a-z0-9_]+\b/i.test(content) && !allowedProjectionPatterns.some((pattern) => pattern.test(content))) {
			errors.push(`${file}: Drive migrations may only keep Billing entitlement projection tables`);
		}
	}
}

function checkInternalAdminBillingBoundary() {
	const commandCenter = "apps/internal-admin/src/internal_admin.command_center.rs";
	if (existsSync(commandCenter)) {
		const content = readFileSync(commandCenter, "utf8");
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${commandCenter}: command center Billing metrics must come from billing-api/gRPC`);
		}
		if (!content.includes("get_admin_command_center_billing_metrics")) {
			errors.push(`${commandCenter}: command center must use Billing gRPC metrics`);
		}
	}

	const operationsCenter = "apps/internal-admin/src/internal_admin.operations_center.rs";
	if (existsSync(operationsCenter)) {
		const content = readFileSync(operationsCenter, "utf8");
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${operationsCenter}: operations center Billing snapshot must come from billing-api/gRPC`);
		}
		if (!content.includes("get_admin_operations_center")) {
			errors.push(`${operationsCenter}: operations center must use Billing gRPC snapshot`);
		}
	}

	const operationsMutations = "apps/internal-admin/src/internal_admin.operations_center.mutations.rs";
	if (existsSync(operationsMutations)) {
		const content = readFileSync(operationsMutations, "utf8");
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${operationsMutations}: operations center Billing mutations must call billing-api/gRPC`);
		}
		if (!content.includes("run_admin_operations_action")) {
			errors.push(`${operationsMutations}: operations center Billing mutations must use Billing gRPC actions`);
		}
	}

	for (const file of [
		"apps/internal-admin/src/internal_admin.billing_platform_center.mutations.rs",
		"apps/internal-admin/src/internal_admin.billing_platform_center.routing_mutations.rs",
		"apps/internal-admin/src/internal_admin.billing_fraud_review.actions.rs",
	]) {
		if (!existsSync(file)) continue;
		const content = readFileSync(file, "utf8");
		if (/\b(?:FROM|JOIN|INTO|UPDATE|TABLE)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${file}: Billing Platform mutations must call billing-api/gRPC`);
		}
		if (!content.includes("run_billing_platform_action")) {
			errors.push(`${file}: Billing Platform mutations must use Billing gRPC actions`);
		}
	}

	for (const file of [
		"apps/internal-admin/src/internal_admin.billing.admin.mutations.rs",
		"apps/internal-admin/src/internal_admin.billing.admin.financial_actions.rs",
	]) {
		if (!existsSync(file)) continue;
		const content = readFileSync(file, "utf8");
		if (/\b(?:INSERT\s+INTO|UPDATE|DELETE\s+FROM)\s+billing_(?:credit_notes|write_offs|refunds|ledger_entries|adjustments)\b/i.test(content)) {
			errors.push(`${file}: Billing financial admin writes must call billing-api/gRPC`);
		}
	}

	const billingAdminMutations = "apps/internal-admin/src/internal_admin.billing.admin.mutations.rs";
	if (existsSync(billingAdminMutations)) {
		const content = readFileSync(billingAdminMutations, "utf8");
		if (/\b(?:INSERT\s+INTO|UPDATE|DELETE\s+FROM)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${billingAdminMutations}: Billing admin mutations must call billing-api/gRPC`);
		}
	}

	const revenueMutations = "apps/internal-admin/src/internal_admin.revenue_center.mutations.rs";
	if (existsSync(revenueMutations)) {
		const content = readFileSync(revenueMutations, "utf8");
		if (/\b(?:INSERT\s+INTO|UPDATE|DELETE\s+FROM)\s+billing_[a-z0-9_]+\b/i.test(content)) {
			errors.push(`${revenueMutations}: Revenue center Billing mutations must call billing-api/gRPC`);
		}
		if (!content.includes("run_revenue_grpc_action")) {
			errors.push(`${revenueMutations}: Revenue center mutations must use Billing gRPC actions`);
		}
	}
}

function checkBillingProviderNeutralSurface() {
	const requiredProviderFiles = [
		["libs/rust/billing/src/provider.rs", 'PROVIDER_CODES: &[&str] = &["stripe", "mollie", "cb"]'],
		["libs/ts/billing-client/src/billing.provider.ts", "['stripe', 'mollie', 'cb']"],
		["apps/gateway-graphql/src/gateway.schema.enums.rs", "Cb"],
		["contracts/graphql/schema.graphql", "CB"],
		["apps/billing-api/migrations/0009_billing_provider_cb.sql", "ADD VALUE IF NOT EXISTS 'cb'"],
	];
	for (const [file, expected] of requiredProviderFiles) {
		if (!existsSync(file)) {
			errors.push(`${file}: required for provider-neutral Billing surface`);
			continue;
		}
		if (!readFileSync(file, "utf8").includes(expected)) {
			errors.push(`${file}: Billing provider-neutral surface must include CB provider evidence ${expected}`);
		}
	}

	for (const file of [
		"contracts/events/billing.payment.changed.v1.schema.json",
		"contracts/events/billing.reconciliation.difference.v1.schema.json",
	]) {
		if (!existsSync(file)) {
			errors.push(`${file}: required for Billing event provider contracts`);
			continue;
		}
		const content = readFileSync(file, "utf8");
		if (!content.includes('"enum": ["stripe", "mollie", "cb"]')) {
			errors.push(`${file}: Billing event provider contract must include stripe, mollie and cb`);
		}
	}
}

function checkBillingMultiPspContinuityEvidence() {
	const providerSubscriptions = "libs/rust/billing/src/db.provider_subscriptions.rs";
	if (!existsSync(providerSubscriptions)) {
		errors.push(`${providerSubscriptions}: required for multi-PSP subscription continuity`);
	} else {
		const content = readFileSync(providerSubscriptions, "utf8");
		for (const expected of [
			"provider_subscription_fallback_eligible",
			"active_non_primary_provider_subscription_remains_fallback_eligible",
			"primary_provider_subscription_is_never_marked_as_fallback",
			"inactive_non_primary_provider_subscription_is_not_fallback_eligible",
			"demoted_primary",
			"fallback_eligible",
		]) {
			if (!content.includes(expected)) {
				errors.push(`${providerSubscriptions}: missing multi-PSP continuity evidence ${expected}`);
			}
		}
	}

	const workspaceEffects = "libs/rust/billing/src/stripe_webhook_workspace_effects.rs";
	if (!existsSync(workspaceEffects)) {
		errors.push(`${workspaceEffects}: required for primary-provider webhook workspace effects`);
	} else {
		const content = readFileSync(workspaceEffects, "utf8");
		for (const expected of [
			"provider_subscription_applies_workspace_effects",
			"workspace_effects_apply_only_to_primary_provider_subscription",
		]) {
			if (!content.includes(expected)) {
				errors.push(`${workspaceEffects}: missing primary-provider workspace effect evidence ${expected}`);
			}
		}
	}
}

checkRustProductPackages();
checkProductSourceImports();
checkIdentityBillingRuntimeBoundary(errors);
checkBillingWorkerQueueBoundaries(errors);
checkIdentityWebBillingClientBoundary(errors);
checkDriveBillingBoundary();
checkDriveBillingMigrationBoundary();
checkInternalAdminBillingBoundary();
checkBillingMigrationBoundaries(errors);
checkBillingProviderNeutralSurface();
checkBillingMultiPspContinuityEvidence();

if (errors.length > 0) {
	console.error("Product boundary violations:");
	for (const error of errors) {
		console.error(`- ${error}`);
	}
	process.exit(1);
}

console.log("Product boundaries: ok");
