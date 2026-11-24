import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const skippedDirs = new Set([".git", ".nx", "coverage", "dist", "node_modules", "target"]);

export function checkBillingMigrationBoundaries(errors) {
	checkIdentityMigrationsDoNotOwnBillingRuntime(errors);
	checkBillingMigrationPipeline(errors);
	checkIdentityMigrationScriptsDoNotTargetBilling(errors);
}

function walk(dir, results = []) {
	if (!existsSync(dir)) return results;
	for (const entry of readdirSync(dir)) {
		const path = join(dir, entry);
		const stat = statSync(path);
		if (stat.isDirectory()) {
			if (!skippedDirs.has(entry)) walk(path, results);
			continue;
		}
		results.push(relative(process.cwd(), path).replaceAll("\\", "/"));
	}
	return results;
}

function checkIdentityMigrationsDoNotOwnBillingRuntime(errors) {
	for (const legacyBillingMigration of [
		"apps/account-service/migrations/0016_billing_platform_core.sql",
		"apps/account-service/migrations/0017_regional_price_mappings.sql",
		"apps/account-service/migrations/0020_internal_admin_entitlement_actions.sql",
		"apps/account-service/migrations/0022_internal_admin_usage_actions.sql",
		"apps/account-service/migrations/0027_internal_admin_risk_actions.sql",
		"apps/account-service/migrations/0028_internal_admin_operations_actions.sql",
		"apps/account-service/migrations/0029_internal_admin_revenue_actions.sql",
		"apps/account-service/migrations/0030_internal_admin_billing_platform_actions.sql",
	]) {
		if (existsSync(legacyBillingMigration)) {
			errors.push(`${legacyBillingMigration}: legacy Billing migration must move to apps/billing-service/migrations`);
		}
	}
	const billingMigrationPatterns = [
		/\bbilling_[a-z0-9_]+\b/i,
		/\bbilling_provider\b/i,
		/\bstripe_price_mappings\b/i,
		/\bsubscription_status\b/i,
		/\bcustomer_type\b/i,
		/\bbilling_adjustment_type\b/i,
		/\bbilling_webhook_status\b/i,
		/\bsubscriptions\b/i,
		/\bbilling_accounts\b/i,
		/\binvoice_estimates\b/i,
		/\busage_snapshots\b/i,
		/\bbilling_webhook_events\b/i,
		/\binternal_admin_usage_actions\b/i,
		/\binternal_admin_entitlement_actions\b/i,
	];
	for (const file of walk("apps/account-service/migrations").filter((path) => path.endsWith(".sql"))) {
		const content = readFileSync(file, "utf8").replace(/\bbilling_admin\b/g, "identity_billing_role");
		if (billingMigrationPatterns.some((pattern) => pattern.test(content))) {
			errors.push(`${file}: Billing runtime migrations must live in apps/billing-service/migrations, not Identity`);
		}
	}
}

function checkBillingMigrationPipeline(errors) {
	const rootPackage = JSON.parse(readFileSync("package.json", "utf8"));
	if (rootPackage?.scripts?.["db:migrate:billing"] !== "bash scripts/db-migrate-billing.sh") {
		errors.push("package.json: db:migrate:billing must use scripts/db-migrate-billing.sh");
	}
	if (rootPackage?.scripts?.["db:migrate"] === rootPackage?.scripts?.["db:migrate:billing"]) {
		errors.push("package.json: db:migrate must not alias the Billing migration pipeline");
	}
	const billingMigrateScript = "scripts/db-migrate-billing.sh";
	if (!existsSync(billingMigrateScript)) {
		errors.push(`${billingMigrateScript}: missing Billing migration script`);
		return;
	}
	const content = readFileSync(billingMigrateScript, "utf8");
	for (const expected of ["NVBES_BILLING_DATABASE_URL", "cargo run -p nvbes-billing-service -- migrate"]) {
		if (!content.includes(expected)) {
			errors.push(`${billingMigrateScript}: Billing migrations must run through billing-service with ${expected}`);
		}
	}
}

function checkIdentityMigrationScriptsDoNotTargetBilling(errors) {
	for (const script of ["scripts/db-migrate.sh"]) {
		if (!existsSync(script)) continue;
		const content = readFileSync(script, "utf8");
		for (const forbidden of ["NVBES_BILLING_DATABASE_URL", "nvbes-billing-service", "apps/billing-service/migrations"]) {
			if (content.includes(forbidden)) {
				errors.push(`${script}: Identity/default migration script must not own Billing migrations (${forbidden})`);
			}
		}
	}
}
