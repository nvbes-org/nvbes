export interface Env {
  SCALEWAY_SECRET_KEY?: string;
  SCALEWAY_PROJECT_ID?: string;
  EMAIL_AUTH_BEARER?: string;
}

export interface EmailRecipient {
  email: string;
  name?: string;
}

export interface EmailDispatchPayload {
  to: EmailRecipient[];
  from_email?: string;
  from_name?: string;
  subject: string;
  html_body: string;
  text_body?: string;
  business_type?: string;
}

export default {
  async fetch(request: Request, env: Env): Promise<Response> {
    const url = new URL(request.url);

    if (request.method === "GET" && url.pathname === "/health") {
      return new Response(JSON.stringify({ status: "healthy", service: "nvbes-email-worker" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      });
    }

    if (request.method !== "POST") {
      return new Response(JSON.stringify({ error: "method_not_allowed" }), {
        status: 405,
        headers: { "Content-Type": "application/json" },
      });
    }

    // Optional Bearer Token Authorization check
    if (env.EMAIL_AUTH_BEARER) {
      const authHeader = request.headers.get("Authorization");
      if (!authHeader || authHeader !== `Bearer ${env.EMAIL_AUTH_BEARER}`) {
        return new Response(JSON.stringify({ error: "unauthorized" }), {
          status: 401,
          headers: { "Content-Type": "application/json" },
        });
      }
    }

    if (url.pathname === "/send") {
      return handleSendEmail(request, env);
    }

    if (url.pathname === "/webhooks/scaleway") {
      return handleScalewayWebhook(request, env);
    }

    return new Response(JSON.stringify({ error: "not_found" }), {
      status: 404,
      headers: { "Content-Type": "application/json" },
    });
  },
};

async function handleSendEmail(request: Request, env: Env): Promise<Response> {
  try {
    const payload: EmailDispatchPayload = await request.json();

    if (!payload.to || payload.to.length === 0 || !payload.subject || !payload.html_body) {
      return new Response(JSON.stringify({ error: "invalid_payload", message: "Missing required fields (to, subject, html_body)" }), {
        status: 400,
        headers: { "Content-Type": "application/json" },
      });
    }

    const scwSecretKey = env.SCALEWAY_SECRET_KEY;
    const scwProjectId = env.SCALEWAY_PROJECT_ID;

    if (!scwSecretKey || !scwProjectId) {
      // In dev / unconfigured mode, log and simulate successful dispatch
      console.log(`[Dev Simulation] Dispatching email to ${payload.to[0].email}: "${payload.subject}"`);
      return new Response(
        JSON.stringify({
          status: "simulated_sent",
          provider: "mock",
          to: payload.to[0].email,
          subject: payload.subject,
        }),
        { status: 200, headers: { "Content-Type": "application/json" } }
      );
    }

    // Call Scaleway TEM REST API
    const scwResponse = await fetch("https://api.scaleway.com/transactional-email/v1alpha1/regions/fr-par/emails", {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        "X-Auth-Token": scwSecretKey,
      },
      body: JSON.stringify({
        project_id: scwProjectId,
        from: {
          email: payload.from_email || "team@nvbes.fr",
          name: payload.from_name || "nvbes",
        },
        to: payload.to,
        subject: payload.subject,
        html: payload.html_body,
        text: payload.text_body,
      }),
    });

    if (!scwResponse.ok) {
      const errorText = await scwResponse.text();
      console.error("[Scaleway TEM Error]", scwResponse.status, errorText);
      return new Response(JSON.stringify({ error: "scaleway_tem_dispatch_failed", details: errorText }), {
        status: scwResponse.status,
        headers: { "Content-Type": "application/json" },
      });
    }

    const scwResult = await scwResponse.json();
    return new Response(JSON.stringify({ status: "sent", provider: "scaleway", result: scwResult }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  } catch (error: any) {
    console.error("[Email Worker Exception]", error);
    return new Response(JSON.stringify({ error: "internal_error", message: error.message || String(error) }), {
      status: 500,
      headers: { "Content-Type": "application/json" },
    });
  }
}

async function handleScalewayWebhook(request: Request, _env: Env): Promise<Response> {
  const event = await request.json();
  console.log("[Scaleway Webhook Event Received]", JSON.stringify(event));
  return new Response(JSON.stringify({ status: "webhook_received" }), {
    status: 200,
    headers: { "Content-Type": "application/json" },
  });
}
