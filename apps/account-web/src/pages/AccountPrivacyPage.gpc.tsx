import type { GpcStatus } from '@nvbes/identity-client';
import { Eye } from 'lucide-react';

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';

export function GpcStatusAlert({ gpc }: { gpc: GpcStatus | null }) {
  if (!gpc?.gpc_enabled) {
    return null;
  }

  return (
    <Alert>
      <Eye />
      <AlertTitle>Le signal GPC de votre navigateur est respecté</AlertTitle>
      <AlertDescription>
        Votre refus de vente ou de partage des données personnelles a été détecté et enregistré
        automatiquement. nvbes ne vend pas vos données personnelles.
      </AlertDescription>
    </Alert>
  );
}
