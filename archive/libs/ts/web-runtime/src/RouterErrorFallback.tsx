import type { ErrorComponentProps } from '@tanstack/react-router';

export function RouterErrorFallback({ error, reset }: ErrorComponentProps) {
  const message = error instanceof Error ? error.message : 'Une erreur inattendue est survenue';

  return (
    <div className="flex min-h-screen items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(15,23,42,0.08),_transparent_35%),linear-gradient(180deg,_#fbfcfe_0%,_#f3f6fb_100%)] p-8">
      <div className="w-full max-w-md rounded-3xl p-8 text-center">
        <div className="space-y-2 pb-3">
          <h1 className="text-2xl font-semibold text-foreground">Une erreur est survenue</h1>
          <p className="text-sm text-muted-foreground">{message}</p>
        </div>
        {reset ? (
          <button
            type="button"
            onClick={() => reset()}
            className="inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
          >
            Réessayer
          </button>
        ) : null}
      </div>
    </div>
  );
}
