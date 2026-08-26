# Webhook Replay Implementation Plan

> **Historical plan.** Reuse only the minimal provider-neutral capability needed
> by the V1 foundation; do not restore archived product or backoffice scope. See
> the [current direction](../../product/nvbes-product-strategy.md).

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement webhook delivery inspection and replay capabilities in the Developer Console.

**Architecture:** Use the existing `/developer/console/webhooks/deliveries/{deliveryId}/replay` Axum backend handler, expose it through the console-web API client, and render an accordion interface listing delivery logs with auto-polling for pending replay states.

**Tech Stack:** Rust (Axum, sqlx), TypeScript, React, TanStack Query, Tailwind CSS.

---

### Task 1: Backend Integration Test

**Files:**
- Modify: `apps/account-service/src/identity.domains.developer.tests.rs`

- [ ] **Step 1: Write integration test for webhook replay**
  Add `test_webhook_replay_route` to verify the webhook replay route works correctly, including verifying the linked delivery ID.
  ```rust
  #[tokio::test]
  async fn test_webhook_replay_route() {
      let pool = crate::test_support::shared_test_pool();
      crate::test_support::ensure_test_database(&pool).await;

      let tenant_id = uuid::Uuid::new_v4();
      let principal_id = uuid::Uuid::new_v4();
      let now = chrono::Utc::now();

      sqlx::query(
          "INSERT INTO tenants (id, kind, name, slug, status, security_tier, created_at, updated_at)
           VALUES ($1, 'team', 'Webhooks Test Tenant', $2, 'active', 'standard', $3, $3)"
      )
      .bind(tenant_id)
      .bind(format!("webhooks-test-{}", tenant_id))
      .bind(now)
      .execute(&pool)
      .await
      .unwrap();

      sqlx::query(
          "INSERT INTO principals (id, tenant_id, principal_kind, status, display_name, created_at, updated_at)
           VALUES ($1, $2, 'human', 'active', 'Webhooks User', $3, $3)"
      )
      .bind(principal_id)
      .bind(tenant_id)
      .bind(now)
      .execute(&pool)
      .await
      .unwrap();

      sqlx::query(
          "INSERT INTO tenant_memberships (tenant_id, principal_id, principal_kind, role, status, created_at, updated_at)
           VALUES ($1, $2, 'human', 'member', 'active', $3, $3)"
      )
      .bind(tenant_id)
      .bind(principal_id)
      .bind(now)
      .execute(&pool)
      .await
      .unwrap();

      sqlx::query(
          "INSERT INTO developer_role_assignments (tenant_id, principal_id, role, created_at)
           VALUES ($1, $2, 'developer_admin', $3)"
      )
      .bind(tenant_id)
      .bind(principal_id)
      .bind(now)
      .execute(&pool)
      .await
      .unwrap();

      let endpoint_id = uuid::Uuid::new_v4();
      sqlx::query(
          "INSERT INTO developer_webhook_endpoints (id, tenant_id, name, url, signing_secret_ciphertext, signing_secret_last4, created_by, created_at, updated_at)
           VALUES ($1, $2, 'Test Endpoint', 'https://example.com/webhook', 'secret', '1234', $3, $4, $4)"
      )
      .bind(endpoint_id)
      .bind(tenant_id)
      .bind(principal_id)
      .bind(now)
      .execute(&pool)
      .await
      .unwrap();

      let delivery_id = uuid::Uuid::new_v4();
      let event_id = uuid::Uuid::new_v4();
      sqlx::query(
          "INSERT INTO developer_webhook_deliveries (id, endpoint_id, tenant_id, event_type, event_id, status, attempt_count, created_at)
           VALUES ($1, $2, $3, 'user.created', $4, 'failed', 1, $5)"
      )
      .bind(delivery_id)
      .bind(endpoint_id)
      .bind(tenant_id)
      .bind(event_id)
      .bind(now)
      .execute(&pool)
      .await
      .unwrap();

      let state = test_state(&pool).await;

      let auth = crate::http::middleware::jwt::AuthContext {
          user_id: principal_id,
          user_email: "dev@example.com".to_string(),
          display_name: "Dev User".to_string(),
          email_verified_at: None,
          mfa_enabled: false,
          tenant_id: Some(tenant_id),
          organization_id: None,
          workspace_id: None,
          workspace_region: None,
          token_type: "Bearer".to_string(),
          scope: String::new(),
          jti: "test-jti".to_string(),
          session_id: uuid::Uuid::new_v4(),
          acr: None,
          amr: vec![],
          auth_time: None,
          client_id: None,
          cnf_jkt: None,
      };

      let response = super::routes::webhooks::replay_delivery(
          axum::extract::State(state.clone()),
          axum::Extension(auth.clone()),
          axum::extract::Path(delivery_id),
      )
      .await
      .unwrap();

      assert_eq!(response.0.status, "pending");
      assert_eq!(response.0.endpoint_id, endpoint_id);
      assert_eq!(response.0.event_id, event_id);
      assert_eq!(response.0.replayed_from_delivery_id, Some(delivery_id));
  }
  ```

- [ ] **Step 2: Run test to verify it passes**
  Run: `rtk cargo test -p account-service domains::developer::tests::test_webhook_replay_route`
  Expected: PASS

- [ ] **Step 3: Commit**
  Run: `rtk git commit -am "test: add integration test for developer webhook replay route"`

---

### Task 2: Frontend API Integration

**Files:**
- Modify: `apps/console-web/src/developer.api.ts`

- [ ] **Step 1: Export API functions for listing deliveries and replaying a webhook delivery**
  Modify `apps/console-web/src/developer.api.ts` to import:
  ```typescript
    DeveloperWebhookDeliveriesSchema,
    DeveloperWebhookDeliverySchema,
    type DeveloperWebhookDelivery,
  ```
  And add the two functions:
  ```typescript
  export async function listDeveloperConsoleWebhookDeliveries(
    endpointId: string,
    signal?: AbortSignal,
  ): Promise<DeveloperWebhookDelivery[]> {
    const response = await identityHttpClient.get(
      `/developer/console/webhooks/${encodeURIComponent(endpointId)}/deliveries`,
      DeveloperWebhookDeliveriesSchema,
      {
        signal: withTimeoutSignal(signal, 4_000),
      },
    );
    return response.deliveries;
  }

  export async function replayDeveloperConsoleWebhookDelivery(
    deliveryId: string,
  ): Promise<DeveloperWebhookDelivery> {
    return identityHttpClient.post(
      `/developer/console/webhooks/deliveries/${encodeURIComponent(deliveryId)}/replay`,
      DeveloperWebhookDeliverySchema,
      {},
    );
  }
  ```

- [ ] **Step 2: Build/Check TypeScript**
  Run: `pnpm --filter console-web check`
  Expected: PASS

- [ ] **Step 3: Commit**
  Run: `rtk git commit -am "feat: add developer webhooks console api functions"`

---

### Task 3: Expandable Webhooks UI with Deliveries List & Replay Buttons

**Files:**
- Modify: `apps/console-web/src/pages/WebhooksPage.tsx`

- [ ] **Step 1: Implement WebhookDeliveriesList component and update WebhooksPage**
  Modify `WebhooksPage.tsx` to support expandable cards, displaying details of delivery logs and replay controls under each endpoint.
  ```typescript
  import { useState } from 'react';
  import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
  import {
    AlertTriangle,
    Webhook,
    ChevronDown,
    ChevronUp,
    CheckCircle2,
    Clock,
    RotateCcw,
    Loader2,
  } from 'lucide-react';
  import {
    listDeveloperConsoleWebhooks,
    listDeveloperConsoleWebhookDeliveries,
    replayDeveloperConsoleWebhookDelivery,
  } from '../developer.api';
  import { canReplayWebhookDelivery } from './WebhooksPage.helpers';
  import { DeveloperWebhookDelivery } from '../developer.schemas';

  export function WebhooksPage() {
    const webhooksQuery = useQuery({
      queryKey: ['console-webhooks'],
      queryFn: ({ signal }) => listDeveloperConsoleWebhooks(signal),
      staleTime: 30_000,
    });

    const [expandedEndpoints, setExpandedEndpoints] = useState<Record<string, boolean>>({});

    const toggleExpand = (endpointId: string) => {
      setExpandedEndpoints((prev) => ({
        ...prev,
        [endpointId]: !prev[endpointId],
      }));
    };

    if (webhooksQuery.isLoading) {
      return <div className="h-72 animate-pulse rounded-lg border border-border bg-card" />;
    }

    if (webhooksQuery.isError || !webhooksQuery.data) {
      return <WebhooksUnavailable />;
    }

    return (
      <section className="space-y-4">
        <div className="flex items-start gap-3">
          <div className="rounded-md border border-border bg-card p-2">
            <Webhook className="h-5 w-5 text-primary" />
          </div>
          <div>
            <h2 className="text-lg font-semibold">Webhooks</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Monitor endpoints and failed deliveries eligible for replay.
            </p>
          </div>
        </div>
        <div className="grid gap-3">
          {webhooksQuery.data.map((endpoint) => {
            const isExpanded = !!expandedEndpoints[endpoint.id];
            return (
              <article
                key={endpoint.id}
                className="rounded-lg border border-border bg-card overflow-hidden"
              >
                <button
                  type="button"
                  onClick={() => toggleExpand(endpoint.id)}
                  className="w-full flex items-start justify-between p-4 hover:bg-muted/50 transition-colors text-left"
                >
                  <div className="space-y-1">
                    <h3 className="text-sm font-semibold">{endpoint.name}</h3>
                    <p className="break-all font-mono text-xs text-muted-foreground">
                      {endpoint.url}
                    </p>
                    <p className="text-xs text-muted-foreground pt-1">
                      Failed deliveries: {endpoint.failed_delivery_count}
                    </p>
                  </div>
                  <div className="flex items-center gap-3">
                    <span className="rounded-md border border-border bg-background px-2 py-1 text-xs font-medium capitalize">
                      {endpoint.status}
                    </span>
                    {isExpanded ? (
                      <ChevronUp className="h-4 w-4 text-muted-foreground" />
                    ) : (
                      <ChevronDown className="h-4 w-4 text-muted-foreground" />
                    )}
                  </div>
                </button>
                {isExpanded && (
                  <div className="border-t border-border bg-muted/20 p-4">
                    <WebhookDeliveriesList endpointId={endpoint.id} />
                  </div>
                )}
              </article>
            );
          })}
          {webhooksQuery.data.length === 0 ? (
            <p className="rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
              No webhook endpoints are configured for this tenant.
            </p>
          ) : null}
        </div>
      </section>
    );
  }

  function WebhookDeliveriesList({ endpointId }: { endpointId: string }) {
    const queryClient = useQueryClient();

    const deliveriesQuery = useQuery({
      queryKey: ['console-webhook-deliveries', endpointId],
      queryFn: ({ signal }) => listDeveloperConsoleWebhookDeliveries(endpointId, signal),
      staleTime: 5_000,
      refetchInterval: (query) => {
        const hasPending = query.state.data?.some((d) => d.status === 'pending');
        return hasPending ? 2_000 : false;
      },
    });

    const replayMutation = useMutation({
      mutationFn: replayDeveloperConsoleWebhookDelivery,
      onSuccess: () => {
        void queryClient.invalidateQueries({
          queryKey: ['console-webhook-deliveries', endpointId],
        });
        void queryClient.invalidateQueries({
          queryKey: ['console-webhooks'],
        });
      },
    });

    if (deliveriesQuery.isLoading) {
      return (
        <div className="flex items-center gap-2 py-4 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin text-primary" />
          Loading delivery history...
        </div>
      );
    }

    if (deliveriesQuery.isError || !deliveriesQuery.data) {
      return (
        <p className="text-sm text-destructive py-2">
          Could not load delivery history for this endpoint.
        </p>
      );
    }

    if (deliveriesQuery.data.length === 0) {
      return (
        <p className="text-sm text-muted-foreground py-2">
          No delivery attempts logged for this endpoint.
        </p>
      );
    }

    return (
      <div className="space-y-3">
        <h4 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Delivery Attempts
        </h4>
        <div className="overflow-hidden rounded-md border border-border bg-card">
          <div className="divide-y divide-border">
            {deliveriesQuery.data.map((delivery) => {
              const replayable = canReplayWebhookDelivery(delivery);
              return (
                <div
                  key={delivery.id}
                  className="flex flex-wrap items-center justify-between gap-4 p-3 hover:bg-muted/30 transition-colors text-sm"
                >
                  <div className="flex items-start gap-3 min-w-0 flex-1">
                    <div className="mt-0.5">
                      <DeliveryStatusIcon status={delivery.status} />
                    </div>
                    <div className="min-w-0 space-y-1">
                      <div className="flex items-center gap-2 flex-wrap">
                        <span className="font-semibold text-foreground">{delivery.event_type}</span>
                        <span className="text-xs font-mono text-muted-foreground break-all">
                          id: {delivery.event_id}
                        </span>
                      </div>
                      <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground">
                        <span>Attempts: {delivery.attempt_count}</span>
                        {delivery.response_status && (
                          <span
                            className={
                              delivery.response_status >= 200 && delivery.response_status < 300
                                ? 'text-green-600 font-medium'
                                : 'text-destructive font-medium'
                            }
                          >
                            HTTP {delivery.response_status}
                          </span>
                        )}
                        <span>{new Date(delivery.created_at).toLocaleString()}</span>
                      </div>
                      {delivery.error_message && (
                        <p className="text-xs text-destructive bg-destructive/5 rounded px-2 py-1 mt-1 font-mono">
                          {delivery.error_message}
                        </p>
                      )}
                      {delivery.replayed_from_delivery_id && (
                        <p className="text-[10px] text-muted-foreground font-mono">
                          Replayed from attempt: {delivery.replayed_from_delivery_id}
                        </p>
                      )}
                    </div>
                  </div>
                  <div>
                    {replayable ? (
                      <button
                        type="button"
                        onClick={() => replayMutation.mutate(delivery.id)}
                        disabled={replayMutation.isPending}
                        className="inline-flex items-center gap-1.5 rounded-md border border-input bg-background px-2.5 py-1 text-xs font-semibold text-foreground shadow-sm hover:bg-muted disabled:opacity-50"
                      >
                        {replayMutation.isPending &&
                        replayMutation.variables === delivery.id ? (
                          <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
                        ) : (
                          <RotateCcw className="h-3 w-3 text-muted-foreground" />
                        )}
                        Replay
                      </button>
                    ) : (
                      <span className="text-xs text-muted-foreground font-medium pr-2 capitalize select-none">
                        {delivery.status}
                      </span>
                    )}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    );
  }

  function DeliveryStatusIcon({ status }: { status: DeveloperWebhookDelivery['status'] }) {
    switch (status) {
      case 'delivered':
        return <CheckCircle2 className="h-4 w-4 text-green-500" />;
      case 'failed':
        return <AlertTriangle className="h-4 w-4 text-destructive" />;
      case 'pending':
        return <Loader2 className="h-4 w-4 text-yellow-500 animate-spin" />;
      case 'replayed':
        return <RotateCcw className="h-4 w-4 text-muted-foreground" />;
      default:
        return <Clock className="h-4 w-4 text-muted-foreground" />;
    }
  }

  function WebhooksUnavailable() {
    return (
      <section className="rounded-lg border border-border bg-card p-6">
        <div className="flex items-center gap-3 text-red-600">
          <AlertTriangle className="h-5 w-5" />
          <h2 className="text-base font-semibold">Webhooks unavailable</h2>
        </div>
        <p className="mt-2 text-sm text-muted-foreground">
          The console could not load webhook endpoints.
        </p>
      </section>
    );
  }
  ```

- [ ] **Step 2: Run frontend test to verify they still pass**
  Run: `pnpm --filter console-web test`
  Expected: PASS

- [ ] **Step 3: Commit changes**
  Run: `rtk git commit -am "feat: implement expandable webhooks UI with replay functionality"`
