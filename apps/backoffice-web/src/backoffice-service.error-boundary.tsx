import { Component, type ErrorInfo, type ReactNode } from 'react';
import { Button } from '@/components/ui/button';

type ErrorBoundaryProps = {
  children: ReactNode;
};

type ErrorBoundaryState = {
  error: Error | null;
};

export class BackofficeServiceErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('backoffice-web crashed', { error, info });
  }

  render() {
    if (!this.state.error) return this.props.children;
    return (
      <main className="bg-background text-foreground flex min-h-screen items-center justify-center p-6">
        <section className="border-border bg-card max-w-md rounded-lg border p-5">
          <h1 className="text-base font-semibold">Back-office indisponible</h1>
          <p className="text-muted-foreground mt-2 text-sm">{this.state.error.message}</p>
          <Button className="mt-4" onClick={() => this.setState({ error: null })} type="button">
            Reessayer
          </Button>
        </section>
      </main>
    );
  }
}
