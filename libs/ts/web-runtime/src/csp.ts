export const permissionsPolicy =
	"accelerometer=(), camera=(), geolocation=(), gyroscope=(), magnetometer=(), microphone=(), payment=(), usb=()";
export const integrityPolicyScripts = "blocked-destinations=(script)";
export const zodJitlessBootstrap =
	"globalThis.__zod_globalConfig ??= {}; globalThis.__zod_globalConfig.jitless = true;";
const zodJitlessBootstrapHash =
	"'sha256-MADsBPBvAfzKSGq+N7sBxSyONZL+4BuiYOfl+I7SpAE='";
export const strictTransportSecurity =
	"max-age=63072000; includeSubDomains; preload";
export const browserIsolationHeaders = {
	"Cross-Origin-Embedder-Policy": "require-corp",
	"Cross-Origin-Opener-Policy": "same-origin",
	"Cross-Origin-Resource-Policy": "same-origin",
	"X-Content-Type-Options": "nosniff",
} as const;
export const uaClientHintsHeaders = {
	"Accept-CH":
		"Sec-CH-UA, Sec-CH-UA-Arch, Sec-CH-UA-Bitness, Sec-CH-UA-Full-Version, Sec-CH-UA-Full-Version-List, Sec-CH-UA-Model, Sec-CH-UA-WoW64, Sec-CH-UA-Form-Factors, Sec-CH-UA-Mobile, Sec-CH-UA-Platform, Sec-CH-UA-Platform-Version",
} as const;

export interface WebCspOptions {
	mode: string;
	nonce?: string;
	scriptSrc?: string[];
	styleSrc?: string[];
	imgSrc?: string[];
	fontSrc?: string[];
	connectSrc?: string[];
	frameSrc?: string[];
	reportUri?: string;
}

const DEV_CONNECT_SOURCES = [
	"ws://localhost:*",
	"ws://127.0.0.1:*",
	"http://localhost:*",
	"http://127.0.0.1:*",
];

export function buildWebCsp(options: WebCspOptions): string {
	const isDev = options.mode === "development";
	const nonceSource = normalizeNonce(options.nonce);
	const reportUri = options.reportUri ?? "/csp-report";
	const directives = [
		directive("default-src", ["'self'"]),
		directive("script-src", [
			"'self'",
			...(!isDev ? [zodJitlessBootstrapHash] : []),
			...(nonceSource ? [nonceSource, "'strict-dynamic'"] : []),
			...(isDev ? ["'unsafe-inline'", "'unsafe-eval'"] : []),
			...(options.scriptSrc ?? []),
		]),
		directive("script-src-attr", ["'none'"]),
		directive("worker-src", ["'self'", "blob:"]),
		directive("style-src", [
			"'self'",
			...(isDev ? ["'unsafe-inline'"] : []),
			...(options.styleSrc ?? []),
		]),
		directive("style-src-elem", [
			"'self'",
			"'unsafe-inline'",
			...(options.styleSrc ?? []),
		]),
		directive("style-src-attr", ["'unsafe-inline'"]),
		directive("img-src", [
			"'self'",
			"data:",
			"blob:",
			...(options.imgSrc ?? []),
		]),
		directive("font-src", ["'self'", "data:", ...(options.fontSrc ?? [])]),
		directive("connect-src", [
			"'self'",
			...(isDev ? DEV_CONNECT_SOURCES : []),
			...(options.connectSrc ?? []),
		]),
		directive("frame-src", ["'self'", ...(options.frameSrc ?? [])]),
		directive("object-src", ["'none'"]),
		directive("base-uri", ["'self'"]),
		directive("form-action", ["'self'"]),
		directive("frame-ancestors", ["'none'"]),
		directive("require-trusted-types-for", ["'script'"]),
		directive("trusted-types", ["nvbes#default", "default"]),
		reportUri ? directive("report-uri", [reportUri]) : "",
	].filter(Boolean);

	return `${directives.join("; ")};`;
}

export function productionTransportHeaders(
	mode: string,
): Readonly<Record<string, string>> {
	return mode === "production"
		? { "Strict-Transport-Security": strictTransportSecurity }
		: {};
}

function normalizeNonce(nonce: string | undefined): string {
	if (!nonce) {
		return "";
	}

	const normalized = nonce.trim();
	if (!/^[A-Za-z0-9+/_-]{16,256}={0,2}$/u.test(normalized)) {
		throw new Error("CSP nonce must be an unquoted base64 or base64url value");
	}

	return `'nonce-${normalized}'`;
}

export function cspMetaFromHeader(csp: string): string {
	return csp
		.replace(/;\s*frame-ancestors 'none'/u, "")
		.replace(/;\s*report-uri [^;]+/u, "")
		.replace(/;\s*report-to [^;]+/u, "");
}

export function originFromUrl(value: string): string {
	try {
		return value ? new URL(value).origin : "";
	} catch {
		return "";
	}
}

export function posthogAssetsOriginFromHost(value: string): string {
	try {
		if (!value) {
			return "";
		}

		const url = new URL(value);
		const cloudAssetsHost = {
			"eu.i.posthog.com": "eu-assets.i.posthog.com",
		}[url.hostname];

		if (cloudAssetsHost) {
			url.hostname = cloudAssetsHost;
			url.port = "";
		}

		return url.origin;
	} catch {
		return "";
	}
}

function directive(name: string, values: string[]): string {
	const uniqueValues = [...new Set(values.filter(Boolean))];
	return `${name} ${uniqueValues.join(" ")}`;
}
