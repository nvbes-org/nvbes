import { useApiStatus } from '../hooks/useApiStatus';
import { WifiOff, RefreshCw, AlertCircle } from 'lucide-react';
import { Badge } from './ui/badge';
import { Button } from './ui/button';
import { Card, CardContent, CardHeader, CardTitle } from './ui/card';
import { Spinner } from './ui/spinner';

export function ApiConnectionOverlay() {
  const { status, checkConnection } = useApiStatus();

  if (status === 'connected') {
    return null;
  }

  const isChecking = status === 'checking';

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4 select-none">
      <div className="absolute inset-0 bg-background/60 backdrop-blur-xl transition-all duration-500" />

      <Card className="relative w-full max-w-md overflow-hidden bg-card/90 shadow-2xl backdrop-blur-md animate-in fade-in-0 zoom-in-95 duration-300">
        <CardHeader className="items-center text-center">
          <div className="relative mb-6 flex h-20 w-20 items-center justify-center rounded-2xl bg-destructive/10 text-destructive ring-1 ring-destructive/20 shadow-[0_0_20px_rgba(239,68,68,0.15)]">
            <WifiOff className="size-10 animate-bounce" />
            <span className="absolute -top-1 -right-1 flex h-4 w-4">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-destructive opacity-75" />
              <span className="relative inline-flex h-4 w-4 rounded-full bg-destructive items-center justify-center">
                <AlertCircle className="size-2.5 text-destructive-foreground" />
              </span>
            </span>
          </div>

          <CardTitle>Service Indisponible</CardTitle>
          <p className="mt-3 text-muted-foreground text-sm leading-relaxed max-w-xs">
            Impossible de joindre nos serveurs d'authentification. Veuillez vérifier votre connexion
            ou réessayer ultérieurement.
          </p>
        </CardHeader>

        <CardContent className="flex flex-col items-center">
          <Badge variant="outline" className="gap-2">
            {isChecking ? <Spinner className="size-3" /> : null}
            {isChecking ? 'Vérification en cours...' : 'Tentative de reconnexion automatique...'}
          </Badge>

          <div className="mt-8 w-full">
            <Button
              onClick={() => checkConnection()}
              disabled={isChecking}
              size="lg"
              variant="destructive"
              className="w-full"
            >
              {isChecking ? (
                <>
                  <RefreshCw data-icon="inline-start" className="animate-spin" />
                  Vérification...
                </>
              ) : (
                <>
                  <RefreshCw data-icon="inline-start" />
                  Réessayer maintenant
                </>
              )}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}
