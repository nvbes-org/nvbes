import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertTriangle, ShieldCheck, Store } from 'lucide-react';
import { useState } from 'react';
import {
  getDeveloperContext,
  listDeveloperMarketplaceApps,
  reviewDeveloperMarketplaceApp,
} from '../developer.api';
import { canUseDeveloperPermission } from '../developer.permissions';
import type { DeveloperMarketplaceApp } from '../developer.schemas';

const badgeClasses: Record<DeveloperMarketplaceApp['status'], string> = {
  pending:
    'bg-amber-50 text-amber-700 border-amber-200 dark:bg-amber-500/10 dark:text-amber-500 dark:border-amber-500/20',
  approved:
    'bg-emerald-50 text-emerald-700 border-emerald-200 dark:bg-emerald-500/10 dark:text-emerald-500 dark:border-emerald-500/20',
  rejected:
    'bg-red-50 text-red-700 border-red-200 dark:bg-red-500/10 dark:text-red-500 dark:border-red-500/20',
  suspended:
    'bg-rose-50 text-rose-700 border-rose-200 dark:bg-rose-500/10 dark:text-rose-500 dark:border-rose-500/20',
};

export function MarketplacePage() {
  const contextQuery = useQuery({
    queryKey: ['developer-context'],
    queryFn: ({ signal }) => getDeveloperContext(signal),
    staleTime: 60_000,
  });

  const appsQuery = useQuery({
    queryKey: ['developer-marketplace-apps'],
    queryFn: ({ signal }) => listDeveloperMarketplaceApps(signal),
    staleTime: 30_000,
  });

  if (appsQuery.isLoading || contextQuery.isLoading) {
    return <div className="h-72 animate-pulse rounded-lg border border-border bg-card" />;
  }

  if (appsQuery.isError || !appsQuery.data || contextQuery.isError || !contextQuery.data) {
    return <MarketplaceUnavailable />;
  }

  const canReview = canUseDeveloperPermission(contextQuery.data, 'marketplace.review');

  return (
    <section className="space-y-4">
      <div className="flex items-start gap-3">
        <div className="rounded-md border border-border bg-card p-2">
          <Store className="h-5 w-5 text-primary" />
        </div>
        <div>
          <h2 className="text-lg font-semibold">OAuth App Marketplace</h2>
          <p className="mt-1 text-sm text-muted-foreground">
            Track OAuth apps awaiting approval or already authorized for internal use.
          </p>
        </div>
      </div>
      <div className="grid gap-3">
        {appsQuery.data.map((app) => (
          <article key={app.client_id} className="rounded-lg border border-border bg-card p-4">
            <div className="flex flex-wrap items-start justify-between gap-3">
              <div>
                <h3 className="text-sm font-semibold">{app.name}</h3>
                <p className="mt-1 font-mono text-xs text-muted-foreground">{app.client_id}</p>
              </div>
              <span
                className={`rounded-md border px-2.5 py-0.5 text-xs font-semibold capitalize ${badgeClasses[app.status]}`}
              >
                {app.status}
              </span>
            </div>
            {app.review_reason ? (
              <div className="mt-3 rounded-md bg-muted/50 p-3 text-sm border border-border/50">
                <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wider mb-1">
                  Review Feedback
                </p>
                <p className="text-muted-foreground">{app.review_reason}</p>
              </div>
            ) : null}
            {canReview ? <ReviewControls app={app} /> : null}
          </article>
        ))}
        {appsQuery.data.length === 0 ? (
          <p className="rounded-lg border border-border bg-card p-6 text-sm text-muted-foreground">
            No OAuth apps are currently submitted to the marketplace.
          </p>
        ) : null}
      </div>
    </section>
  );
}

function ReviewControls({ app }: { app: DeveloperMarketplaceApp }) {
  const queryClient = useQueryClient();
  const [reason, setReason] = useState('');

  const reviewMutation = useMutation({
    mutationFn: ({ status, reviewReason }: { status: string; reviewReason?: string }) =>
      reviewDeveloperMarketplaceApp(app.client_id, status, reviewReason),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: ['developer-marketplace-apps'] });
      void queryClient.invalidateQueries({ queryKey: ['developer-oauth-clients'] });
      void queryClient.invalidateQueries({ queryKey: ['developer-overview'] });
      setReason('');
    },
  });

  const handleReview = (status: string) => {
    reviewMutation.mutate({ status, reviewReason: reason.trim() || undefined });
  };

  return (
    <div className="mt-4 border-t border-border/80 pt-4">
      <div className="flex flex-col gap-3 md:flex-row md:items-end justify-between">
        <div className="flex-1 max-w-lg">
          <label
            htmlFor={`reason-${app.client_id}`}
            className="block text-xs font-semibold text-muted-foreground mb-1.5 uppercase tracking-wide"
          >
            Review Comments / Feedback (optional for approval)
          </label>
          <input
            id={`reason-${app.client_id}`}
            type="text"
            placeholder="Reason for approval, rejection or suspension..."
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            className="w-full rounded-md border border-border bg-background px-3 py-1.5 text-sm placeholder:text-muted-foreground/60 focus:border-primary focus:outline-none transition-colors"
          />
        </div>
        <div className="flex items-center gap-2 flex-wrap">
          {app.status === 'pending' || app.status === 'rejected' || app.status === 'suspended' ? (
            <button
              onClick={() => handleReview('approved')}
              disabled={reviewMutation.isPending}
              className="inline-flex items-center gap-1 rounded-md bg-emerald-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-emerald-700 disabled:opacity-50 transition-colors"
            >
              <ShieldCheck className="h-3.5 w-3.5" />
              Approve
            </button>
          ) : null}
          {app.status === 'pending' ? (
            <button
              onClick={() => handleReview('rejected')}
              disabled={reviewMutation.isPending}
              className="rounded-md bg-red-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-red-700 disabled:opacity-50 transition-colors"
            >
              Reject
            </button>
          ) : null}
          {app.status === 'approved' ? (
            <button
              onClick={() => handleReview('suspended')}
              disabled={reviewMutation.isPending}
              className="rounded-md bg-amber-600 px-3 py-1.5 text-xs font-semibold text-white hover:bg-amber-700 disabled:opacity-50 transition-colors"
            >
              Suspend
            </button>
          ) : null}
        </div>
      </div>
      {reviewMutation.isError ? (
        <p className="mt-2 text-xs text-red-600">Failed to submit review. Please try again.</p>
      ) : null}
    </div>
  );
}

function MarketplaceUnavailable() {
  return (
    <section className="rounded-lg border border-border bg-card p-6">
      <div className="flex items-center gap-3 text-red-600">
        <AlertTriangle className="h-5 w-5" />
        <h2 className="text-base font-semibold">Marketplace unavailable</h2>
      </div>
      <p className="mt-2 text-sm text-muted-foreground">
        The console could not load marketplace approvals.
      </p>
    </section>
  );
}
