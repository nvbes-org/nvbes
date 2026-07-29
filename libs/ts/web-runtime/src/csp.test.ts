import { describe, expect, it } from "vitest";
import { buildWebCsp, strictTransportSecurity } from "./csp";

describe("buildWebCsp", () => {
	it("enforces Trusted Types and denies framing", () => {
		const csp = buildWebCsp({ mode: "production" });

		expect(csp).toContain("frame-ancestors 'none'");
		expect(csp).toContain("require-trusted-types-for 'script'");
		expect(csp).toContain("trusted-types nvbes#default default");
		expect(csp).toContain(
			"'sha256-MADsBPBvAfzKSGq+N7sBxSyONZL+4BuiYOfl+I7SpAE='",
		);
		expect(csp).not.toContain("script-src 'self' 'unsafe-inline'");
	});

	it("supports a per-response nonce with strict-dynamic", () => {
		const csp = buildWebCsp({
			mode: "production",
			nonce: "xF7tB5Jm5V8aQ2kLw9cR3g==",
		});

		expect(csp).toContain("'nonce-xF7tB5Jm5V8aQ2kLw9cR3g=='");
		expect(csp).toContain("'strict-dynamic'");
	});

	it("rejects malformed nonces", () => {
		expect(() =>
			buildWebCsp({ mode: "production", nonce: "'unsafe-inline'" }),
		).toThrow("CSP nonce");
	});

	it("exports a preload-ready HSTS policy", () => {
		expect(strictTransportSecurity).toBe(
			"max-age=63072000; includeSubDomains; preload",
		);
	});
});
