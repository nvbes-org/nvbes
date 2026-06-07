import type { ReactNode } from 'react';

export function DeviceActivationShell({ error, children }: { error: string; children: ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-background p-4">
      <div className="w-full max-w-md space-y-8 rounded-2xl border bg-card p-8 shadow-xl">
        <div className="text-center">
          <h1 className="text-3xl font-extrabold tracking-tight">Activation de l&apos;appareil</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            Connectez votre appareil à votre compte nvbes.
          </p>
        </div>

        {error ? (
          <div className="rounded-lg bg-destructive/10 p-3 text-sm text-destructive">{error}</div>
        ) : null}

        {children}
      </div>
    </div>
  );
}
