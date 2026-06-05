import React from 'react';
import { ErrorBoundary } from './ErrorBoundary';

export interface FeatureFallbackProps {
  error: Error;
  onReset: () => void;
  onReport: () => void;
  canReport: boolean;
  reported: boolean;
}

function FallbackLayout({
  title,
  description,
  error,
  onReset,
  onReport,
  canReport,
  reported,
}: FeatureFallbackProps & { title: string; description: string }) {
  return (
    <div className="flex min-h-[400px] items-center justify-center bg-[radial-gradient(circle_at_top,_rgba(15,23,42,0.08),_transparent_35%),linear-gradient(180deg,_#fbfcfe_0%,_#f3f6fb_100%)] p-8">
      <div className="w-full max-w-md rounded-3xl border border-border/60 bg-background/95 p-8 text-center shadow-2xl shadow-black/5">
        <div className="space-y-2">
          <h1 className="text-2xl font-semibold text-foreground">{title}</h1>
          <p className="text-sm text-muted-foreground">{description}</p>
          {error.message && (
            <details className="mt-2 text-left">
              <summary className="cursor-pointer text-xs text-muted-foreground">
                Details techniques
              </summary>
              <pre className="mt-2 max-h-32 overflow-auto rounded-md border bg-muted p-3 text-xs text-muted-foreground">
                {error.message}
              </pre>
            </details>
          )}
        </div>
        <div className="flex items-center justify-center gap-3">
          <button
            type="button"
            onClick={onReset}
            className="inline-flex items-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90"
          >
            Réessayer
          </button>
          {canReport ? (
            <button
              type="button"
              onClick={onReport}
              disabled={reported}
              className="inline-flex items-center rounded-md border bg-background px-4 py-2 text-sm font-medium text-foreground hover:bg-muted"
            >
              {reported ? 'Signalé' : 'Signaler'}
            </button>
          ) : null}
        </div>
      </div>
    </div>
  );
}

export function AuthErrorFallback({
  error,
  onReset,
  onReport,
  canReport,
  reported,
}: FeatureFallbackProps) {
  return (
    <FallbackLayout
      error={error}
      onReset={onReset}
      onReport={onReport}
      canReport={canReport}
      reported={reported}
      title="Probleme d'authentification"
      description="Impossible de verifier votre identite. Verifiez votre connexion internet."
    />
  );
}

export function BillingErrorFallback({
  error,
  onReset,
  onReport,
  canReport,
  reported,
}: FeatureFallbackProps) {
  return (
    <FallbackLayout
      error={error}
      onReset={onReset}
      onReport={onReport}
      canReport={canReport}
      reported={reported}
      title="Probleme de paiement"
      description="Une erreur est survenue lors du traitement de votre paiement. Aucun montant n'a ete debite."
    />
  );
}

export function UploadErrorFallback({
  error,
  onReset,
  onReport,
  canReport,
  reported,
}: FeatureFallbackProps) {
  return (
    <FallbackLayout
      error={error}
      onReset={onReset}
      onReport={onReport}
      canReport={canReport}
      reported={reported}
      title="Echec du televersement"
      description="Votre fichier n'a pas pu etre televerse. Verifiez votre connexion et l'espace disponible."
    />
  );
}

export function EditorErrorFallback({
  error,
  onReset,
  onReport,
  canReport,
  reported,
}: FeatureFallbackProps) {
  return (
    <FallbackLayout
      error={error}
      onReset={onReset}
      onReport={onReport}
      canReport={canReport}
      reported={reported}
      title="L'editeur a rencontre une erreur"
      description="Vos modifications n'ont pas ete enregistrees. Vous pouvez reessayer ou recharger la page."
    />
  );
}

// ---------------------------------------------------------------------------
// Convenience wrappers — multi-level error boundaries per feature
// ---------------------------------------------------------------------------

function createBoundary(name: string, Fallback: React.ComponentType<FeatureFallbackProps>) {
  return function FeatureBoundary({ children }: { children: React.ReactNode }) {
    return (
      <ErrorBoundary name={name} renderFallback={(props) => <Fallback {...props} />}>
        {children}
      </ErrorBoundary>
    );
  };
}

export const AuthErrorBoundary = createBoundary('auth', AuthErrorFallback);
export const BillingErrorBoundary = createBoundary('billing', BillingErrorFallback);
export const UploadErrorBoundary = createBoundary('upload', UploadErrorFallback);
export const EditorErrorBoundary = createBoundary('editor', EditorErrorFallback);
