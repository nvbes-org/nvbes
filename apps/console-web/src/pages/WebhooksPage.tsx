import { RelativeTime, useVisibilityAwareInterval } from '@nvbes/web-runtime';
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
import { DeveloperWebhookDelivery } from '../developer.schemas';
import { DeveloperVirtualStack } from './DeveloperVirtualStack';
import { canReplayWebhookDelivery } from './WebhooksPage.helpers';

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
  const liveInterval = useVisibilityAwareInterval(2_000, false);

  const deliveriesQuery = useQuery({
    queryKey: ['console-webhook-deliveries', endpointId],
    queryFn: ({ signal }) => listDeveloperConsoleWebhookDeliveries(endpointId, signal),
    staleTime: 5_000,
    refetchInterval: (query) => {
      const hasPending = query.state.data?.some((d) => d.status === 'pending');
      return hasPending ? liveInterval : false;
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
        <DeveloperVirtualStack
          items={deliveriesQuery.data}
          className="max-h-[560px] overflow-auto"
          itemClassName="border-b border-border last:border-b-0"
          estimateSize={112}
          getKey={(delivery) => delivery.id}
          renderItem={(delivery) => {
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
                      <RelativeTime value={delivery.created_at} />
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
                      {replayMutation.isPending && replayMutation.variables === delivery.id ? (
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
          }}
        />
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
