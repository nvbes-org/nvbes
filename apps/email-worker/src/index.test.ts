import { describe, expect, it } from "vitest";
import worker from "./index";

describe("Cloudflare Email Worker Tests", () => {
  const dummyEnv = {
    SCALEWAY_SECRET_KEY: "",
    SCALEWAY_PROJECT_ID: "",
  };

  it("should return 200 OK for GET /health", async () => {
    const request = new Request("http://localhost/health", { method: "GET" });
    const response = await worker.fetch(request, dummyEnv);

    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body).toEqual({ status: "healthy", service: "nvbes-email-worker" });
  });

  it("should return 405 Method Not Allowed for GET /send", async () => {
    const request = new Request("http://localhost/send", { method: "GET" });
    const response = await worker.fetch(request, dummyEnv);

    expect(response.status).toBe(405);
  });

  it("should return 400 Bad Request when POST /send payload is missing fields", async () => {
    const request = new Request("http://localhost/send", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ subject: "Test" }),
    });

    const response = await worker.fetch(request, dummyEnv);
    expect(response.status).toBe(400);
    const body = await response.json();
    expect(body.error).toBe("invalid_payload");
  });

  it("should return 401 Unauthorized when Bearer token is invalid", async () => {
    const envWithAuth = {
      ...dummyEnv,
      EMAIL_AUTH_BEARER: "secret-bearer-123",
    };

    const request = new Request("http://localhost/send", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: "Bearer wrong-token",
      },
      body: JSON.stringify({
        to: [{ email: "user@example.com" }],
        subject: "Test",
        html_body: "<p>Hello</p>",
      }),
    });

    const response = await worker.fetch(request, envWithAuth);
    expect(response.status).toBe(401);
  });

  it("should simulate dispatch when Scaleway keys are not configured", async () => {
    const request = new Request("http://localhost/send", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        to: [{ email: "user@example.com", name: "User" }],
        subject: "Verification Email",
        html_body: "<p>Welcome to nvbes!</p>",
      }),
    });

    const response = await worker.fetch(request, dummyEnv);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.status).toBe("simulated_sent");
    expect(body.to).toBe("user@example.com");
  });

  it("should return 200 OK for POST /webhooks/scaleway", async () => {
    const request = new Request("http://localhost/webhooks/scaleway", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ event: "delivered", email_id: "test-id-123" }),
    });

    const response = await worker.fetch(request, dummyEnv);
    expect(response.status).toBe(200);
    const body = await response.json();
    expect(body.status).toBe("webhook_received");
  });
});
