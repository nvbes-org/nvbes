import { createHttpClient } from "@nvbes/http-client";
import { describe, expect, it } from "vite-plus/test";
import { z } from "zod";

describe("http-client idempotency headers", () => {
	it("adds an Idempotency-Key to POST requests by default", async () => {
		const observed = new Headers();
		const client = createHttpClient({ fetchImpl: buildJsonFetch(observed) });

		await client.post("/mutations", z.object({ ok: z.boolean() }), {
			ok: true,
		});

		expect(observed.get("Idempotency-Key")).toMatch(/^[0-9a-f-]+$/i);
	});

	it("preserves an explicit Idempotency-Key header", async () => {
		const observed = new Headers();
		const client = createHttpClient({ fetchImpl: buildJsonFetch(observed) });

		await client.request("/mutations", z.object({ ok: z.boolean() }), {
			body: { ok: true },
			headers: { "Idempotency-Key": "custom-key-123" },
			method: "PATCH",
		});

		expect(observed.get("Idempotency-Key")).toBe("custom-key-123");
	});

	it("adds an Idempotency-Key to PUT requests by default", async () => {
		const observed = new Headers();
		const client = createHttpClient({ fetchImpl: buildJsonFetch(observed) });

		await client.request("/mutations", z.object({ ok: z.boolean() }), {
			body: { ok: true },
			method: "PUT",
		});

		expect(observed.get("Idempotency-Key")).toMatch(/^[0-9a-f-]+$/i);
	});

	it("can disable automatic Idempotency-Key generation per request", async () => {
		const observed = new Headers();
		const client = createHttpClient({ fetchImpl: buildJsonFetch(observed) });

		await client.request("/mutations", z.object({ ok: z.boolean() }), {
			body: { ok: true },
			idempotencyKey: false,
			method: "POST",
		});

		expect(observed.get("Idempotency-Key")).toBeNull();
	});
});

function buildJsonFetch(observed: Headers): typeof fetch {
	return async (_input, init) => {
		const requestHeaders = new Headers(init?.headers);
		requestHeaders.forEach((value, key) => {
			observed.set(key, value);
		});

		return new Response(JSON.stringify({ ok: true }), {
			headers: { "Content-Type": "application/json" },
			status: 200,
		});
	};
}
