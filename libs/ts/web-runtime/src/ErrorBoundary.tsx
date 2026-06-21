import React from 'react';
import { canReportClientError, reportClientError } from './report-error';
import { isSessionStaleError } from './session-stale';

export interface ErrorBoundaryProps {
  children: React.ReactNode;
  fallback?: React.ReactNode;
  name?: string;
  showReport?: boolean;
  onError?: (error: Error, errorInfo: React.ErrorInfo) => void;
  onReport?: (error: Error) => void;
  renderFallback?: (props: {
    error: Error;
    onReset: () => void;
    onReport: () => void;
    canReport: boolean;
    reported: boolean;
  }) => React.ReactNode;
}

interface ErrorBoundaryState {
  error: Error | null;
  reported: boolean;
}

export class ErrorBoundary extends React.Component<ErrorBoundaryProps, ErrorBoundaryState> {
  constructor(props: ErrorBoundaryProps) {
    super(props);
    this.state = { error: null, reported: false };
  }

  static getDerivedStateFromError(error: Error): Partial<ErrorBoundaryState> {
    return { error };
  }

  componentDidCatch(error: Error, errorInfo: React.ErrorInfo): void {
    this.props.onError?.(error, errorInfo);
    this.reportError(error);
  }

  reportError = (error: Error) => {
    const name = this.props.name ?? 'unknown';
    this.props.onReport?.(error);
    void reportClientError(name, error);
    this.setState({ reported: true });
  };

  handleReset = () => {
    this.setState({ error: null, reported: false });
  };

  handleReport = () => {
    if (this.state.error) {
      this.reportError(this.state.error);
    }
  };

  render() {
    if (this.state.error) {
      const handlers = {
        onReset: this.handleReset,
        onReport: this.handleReport,
      };
      const canReport = this.props.showReport ?? canReportClientError();

      if (this.props.renderFallback) {
        return this.props.renderFallback({
          error: this.state.error,
          canReport,
          reported: this.state.reported,
          ...handlers,
        });
      }
      if (this.props.fallback) {
        return this.props.fallback;
      }
      return (
        <DefaultErrorFallback
          error={this.state.error}
          canReport={canReport}
          reported={this.state.reported}
          {...handlers}
        />
      );
    }
    return this.props.children;
  }
}

function DefaultErrorFallback({
  error,
  canReport,
  reported,
  onReset,
  onReport,
}: {
  error: Error;
  canReport: boolean;
  reported: boolean;
  onReset: () => void;
  onReport: () => void;
}) {
  const sessionStale = isSessionStaleError(error);
  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-8">
      <div className="w-full max-w-md space-y-6 text-center">
        <div className="space-y-2">
          <h1 className="text-2xl font-semibold text-foreground">
            {sessionStale ? 'Session expiree' : 'Une erreur est survenue'}
          </h1>
          <p className="text-sm text-muted-foreground">{error.message}</p>
        </div>
        <div className="flex items-center justify-center gap-3">
          <button
            type="button"
            onClick={onReset}
            className="inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
          >
            {sessionStale ? 'Reprendre la session' : 'Réessayer'}
          </button>
          {canReport ? (
            <button
              type="button"
              onClick={onReport}
              disabled={reported}
              className="inline-flex items-center rounded-md border bg-background px-4 py-2 text-sm font-medium text-foreground hover:bg-muted disabled:opacity-50"
            >
              {reported ? 'Signalé' : 'Signaler'}
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}
