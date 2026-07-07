import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join, relative } from "node:path";

const skippedDirs = new Set([".git", ".nx", "coverage", "dist", "node_modules", "target"]);
const textExtensions = new Set([".ts", ".tsx", ".js", ".jsx", ".mjs", ".html", ".md"]);

function normalizePath(value) {
	return value.replaceAll("\\", "/");
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
			continue;
		}
		const relativePath = normalizePath(relative(process.cwd(), path));
		if (shouldScan(relativePath)) results.push(relativePath);
	}
	return results;
}

export function checkAccountWebBillingClientBoundary(errors) {
	checkDedicatedBillingClient(errors);
	checkAccountWebSourceDoesNotUseAccountBilling(errors);
	checkAccountWebDoesNotExposeBillingRuntime(errors);
	checkAccountWebDoesNotEmitBillingAnalytics(errors);
	checkAccountWebDoesNotLoadPspVendors(errors);
	checkBillingClientDoesNotExposeRawProviderIds(errors);
	checkIdentityClientDoesNotExposeBillingTransport(errors);
}

function checkDedicatedBillingClient(errors) {
	const billingClientPath = "apps/account-web/src/account.billing.client.ts";
	if (!existsSync(billingClientPath)) {
		errors.push(`${billingClientPath}: Account Web must centralize Account-facing Billing transport through a dedicated client`);
		return;
	}

	const content = readFileSync(billingClientPath, "utf8");
	if (!content.includes("identityHttpClient")) {
		errors.push(`${billingClientPath}: Account Web Billing transport must call the Account Service facade`);
	}
	if (!content.includes("/account/billing/workspaces/")) {
		errors.push(`${billingClientPath}: Account Web Billing transport must use /account/billing facade routes`);
	}
	if (content.includes("@nvbes/billing-client") || content.includes("VITE_BILLING_SERVICE_BASE_URL")) {
		errors.push(`${billingClientPath}: Account Web must not call Billing Service directly`);
	}
	if (
		content
			.split(/\r?\n/)
			.some((line) => /[`"']\/workspaces\/[^`"']*\/billing\//.test(line))
	) {
		errors.push(`${billingClientPath}: Account Web must not construct legacy workspace Billing routes`);
	}
}

function checkAccountWebSourceDoesNotUseAccountBilling(errors) {
	const rawProviderIds = [
		"provider_customer_id",
		"provider_payment_method_id",
		"provider_invoice_id",
		"provider_subscription_id",
		"provider_price_id",
		"provider_product_id",
		"stripe_customer_id",
		"stripe_price_id",
		"billing_customer_id",
		"billing_subscription_id",
	];

	for (const file of walk("apps/account-web/src")) {
		const content = readFileSync(file, "utf8");
		if (/\bidentityClient\.(?:getBilling|createBilling)[A-Za-z0-9_]*/.test(content)) {
			errors.push(`${file}: Account Web must use the Billing client for Billing calls`);
		}
		for (const forbidden of rawProviderIds) {
			if (content.includes(forbidden)) {
				errors.push(`${file}: Account Web must not expose or track raw Billing provider identifier ${forbidden}`);
			}
		}
	}
}

function checkAccountWebDoesNotExposeBillingRuntime(errors) {
	const forbiddenRuntimeCopy = [/worker billing/i, /billing worker/i, /file de paiement/i];
	for (const file of walk("apps/account-web/src")) {
		const content = readFileSync(file, "utf8");
		for (const forbidden of forbiddenRuntimeCopy) {
			if (forbidden.test(content)) {
				errors.push(`${file}: Account Web must not expose Billing runtime supervision copy ${forbidden}`);
			}
		}
	}
}

function checkAccountWebDoesNotEmitBillingAnalytics(errors) {
	for (const file of walk("apps/account-web/src")) {
		const content = readFileSync(file, "utf8");
		if (/trackEvent\(\s*["']billing\./.test(content)) {
			errors.push(`${file}: Account Web must not emit Billing analytics events; Billing runtime owns Billing telemetry`);
		}
	}
}

function checkAccountWebDoesNotLoadPspVendors(errors) {
	const files = [
		...walk("apps/account-web/src"),
		"apps/account-web/README.md",
		"apps/account-web/index.html",
		"apps/account-web/vite.config.ts",
		"apps/account-web/identity.vite.csp.ts",
	];
	const forbiddenValues = [
		"VITE_STRIPE_",
		"https://js.stripe.com",
		"https://api.stripe.com",
		"js.stripe.com",
		"api.stripe.com",
		"Stripe",
	];

	for (const file of files) {
		if (!existsSync(file)) continue;
		const content = readFileSync(file, "utf8");
		for (const forbidden of forbiddenValues) {
			if (content.includes(forbidden)) {
				errors.push(`${file}: Account Web must not load or configure PSP vendor endpoints directly; use Billing service routes`);
			}
		}
	}
}

function checkBillingClientDoesNotExposeRawProviderIds(errors) {
	const forbiddenProviderIds = [
		"provider_customer_id",
		"provider_payment_method_id",
		"provider_invoice_id",
		"provider_subscription_id",
		"provider_price_id",
		"provider_product_id",
		"stripe_customer_id",
		"stripe_price_id",
		"billing_customer_id",
		"billing_subscription_id",
		"session_id",
		"checkout_id",
		"payment_id",
	];

	for (const file of walk("libs/ts/billing-client/src")) {
		const content = readFileSync(file, "utf8");
		for (const forbidden of forbiddenProviderIds) {
			if (content.includes(forbidden)) {
				errors.push(`${file}: Billing TypeScript client must not expose raw provider identifier ${forbidden}`);
			}
		}
	}
}

function checkIdentityClientDoesNotExposeBillingTransport(errors) {
	const accountClient = "libs/ts/identity-client/src/identity.account-client.ts";
	if (existsSync(accountClient)) {
		const content = readFileSync(accountClient, "utf8");
		if (/\/workspaces\/.*\/billing\//.test(content) || /\b(?:getBilling|createBilling)[A-Za-z0-9_]*/.test(content)) {
			errors.push(`${accountClient}: AccountIdentityClient must not expose Billing transport methods`);
		}
	}

	for (const file of walk("libs/ts/identity-client/src")) {
		const content = readFileSync(file, "utf8");
		if (/billing\.(?:client|provider|schemas)/.test(file) || /@nvbes\/billing-client/.test(content)) {
			errors.push(`${file}: Identity client package must not own or re-export Billing client surfaces`);
		}
		if (/\/enterprise\/billing\b/.test(content) || /\bEnterpriseBilling[A-Za-z0-9_]*\b/.test(content)) {
			errors.push(`${file}: Identity client package must not expose Enterprise Billing transport surfaces`);
		}
	}
}
