import type { ReactNode } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';

export function DeviceActivationShell({ error, children }: { error: string; children: ReactNode }) {
  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-background p-4">
      <Card className="w-full max-w-md">
        <CardHeader className="text-center">
          <CardTitle className="text-3xl">Activation de l&apos;appareil</CardTitle>
          <p className="text-sm text-muted-foreground">
            Connectez votre appareil à votre compte nvbes.
          </p>
        </CardHeader>

        {error ? (
          <CardContent>
            <Alert variant="destructive">
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          </CardContent>
        ) : null}

        <CardContent>{children}</CardContent>
      </Card>
    </div>
  );
}
