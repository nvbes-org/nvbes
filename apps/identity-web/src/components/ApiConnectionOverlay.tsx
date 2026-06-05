import { useApiStatus } from '../hooks/useApiStatus';
import { WifiOff, RefreshCw, AlertCircle } from 'lucide-react';
import { Button } from './ui/button';

export function ApiConnectionOverlay() {
  const { status, checkConnection } = useApiStatus();

  if (status === 'connected') {
    return null;
  }

  const isChecking = status === 'checking';

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 select-none">
      {/* Premium blurred gradient background */}
      <div className="absolute inset-0 bg-background/60 backdrop-blur-xl transition-all duration-500" />

      {/* Decorative premium ambient glowing orbs */}
      <div className="absolute top-1/4 left-1/4 -z-10 h-72 w-72 rounded-full bg-destructive/15 blur-3xl animate-pulse" />
      <div className="absolute bottom-1/4 right-1/4 -z-10 h-80 w-80 rounded-full bg-primary/10 blur-3xl animate-pulse [animation-delay:2s]" />

      {/* Glassmorphic main card */}
      <div className="relative w-full max-w-md overflow-hidden rounded-3xl border border-border/40 bg-card/45 p-8 shadow-2xl backdrop-blur-md animate-in fade-in-0 zoom-in-95 duration-300">
        {/* Glow accent bar at the top */}
        <div className="absolute top-0 inset-x-0 h-1 bg-gradient-to-r from-destructive via-amber-500 to-destructive" />

        <div className="flex flex-col items-center text-center">
          {/* Outer glowing icon container */}
          <div className="relative mb-6 flex h-20 w-20 items-center justify-center rounded-2xl bg-destructive/10 text-destructive ring-1 ring-destructive/20 shadow-[0_0_20px_rgba(239,68,68,0.15)]">
            <WifiOff className="h-10 w-10 animate-bounce" />
            <span className="absolute -top-1 -right-1 flex h-4 w-4">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-destructive opacity-75" />
              <span className="relative inline-flex h-4 w-4 rounded-full bg-destructive items-center justify-center">
                <AlertCircle className="h-2.5 w-2.5 text-white" />
              </span>
            </span>
          </div>

          <h2 className="text-2xl font-bold tracking-tight text-foreground sm:text-3xl">
            Service Indisponible
          </h2>
          <p className="mt-3 text-muted-foreground text-sm leading-relaxed max-w-xs">
            Impossible de joindre nos serveurs d'authentification. Veuillez vérifier votre connexion
            ou réessayer ultérieurement.
          </p>

          {/* Interactive loading / polling state info */}
          <div className="mt-6 flex items-center gap-2 rounded-full border border-border/20 bg-muted/30 px-4 py-1.5 text-xs text-muted-foreground">
            <span className="relative flex h-2 w-2">
              <span
                className={`absolute inline-flex h-full w-full rounded-full bg-amber-500 opacity-75 ${isChecking ? 'animate-ping' : 'animate-pulse'}`}
              />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-amber-500" />
            </span>
            {isChecking ? 'Vérification en cours...' : 'Tentative de reconnexion automatique...'}
          </div>

          <div className="mt-8 w-full">
            <Button
              onClick={() => checkConnection()}
              disabled={isChecking}
              size="lg"
              className="w-full relative overflow-hidden rounded-2xl bg-destructive text-destructive-foreground hover:bg-destructive/90 transition-all shadow-[0_4px_12px_rgba(239,68,68,0.2)] hover:shadow-[0_4px_20px_rgba(239,68,68,0.3)] duration-200 active:scale-[0.98]"
            >
              {isChecking ? (
                <>
                  <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
                  Vérification...
                </>
              ) : (
                <>
                  <RefreshCw className="mr-2 h-4 w-4" />
                  Réessayer maintenant
                </>
              )}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
