import React from 'react';
import { ErrorBoundary } from './ErrorBoundary';

export interface ErrorWithRetryProps {
  error: Error;
  onRetry: () => void;
  canReport?: boolean;
  onReport?: () => void;
  reported?: boolean;
  title?: string;
}

export function ErrorWithRetry({
  error,
  onRetry,
  canReport = false,
  onReport,
  reported = false,
  title = 'Something went wrong',
}: ErrorWithRetryProps) {
  return (
    <div className="rounded-md border border-border bg-card p-4">
      <div className="space-y-1">
        <h2 className="text-sm font-semibold text-foreground">{title}</h2>
        <p className="text-sm text-muted-foreground">{error.message}</p>
      </div>
      <div className="mt-4 flex flex-wrap gap-2">
        <button
          type="button"
          onClick={onRetry}
          className="rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground"
        >
          Retry
        </button>
        {canReport && onReport ? (
          <button
            type="button"
            onClick={onReport}
            disabled={reported}
            className="rounded-md border border-border px-3 py-1.5 text-sm font-medium text-foreground disabled:opacity-50"
          >
            {reported ? 'Reported' : 'Report'}
          </button>
        ) : null}
      </div>
    </div>
  );
}

export interface FeatureBoundaryProps {
  app: string;
  surface: string;
  route?: string;
  operation?: string;
  children: React.ReactNode;
}

export function FeatureBoundary({
  app,
  surface,
  route,
  operation,
  children,
}: FeatureBoundaryProps) {
  const name = [app, surface, route, operation].filter(Boolean).join(':');

  return (
    <ErrorBoundary
      name={name}
      renderFallback={(props) => (
        <ErrorWithRetry
          error={props.error}
          onRetry={props.onReset}
          canReport={props.canReport}
          onReport={props.onReport}
          reported={props.reported}
          title={`${surface} unavailable`}
        />
      )}
    >
      {children}
    </ErrorBoundary>
  );
}
