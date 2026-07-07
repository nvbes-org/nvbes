import { type GpcStatus } from '@nvbes/identity-client';
import { Eye } from 'lucide-react';

import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';

export function GpcStatusCard({ gpc }: { gpc: GpcStatus | null }) {
  if (!gpc?.gpc_enabled) {
    return null;
  }

  return (
    <Card className="animate-fade-slide-up border-emerald-500/20 bg-emerald-500/[0.02] [animation-delay:50ms]">
      <CardHeader>
        <div className="flex items-center gap-2">
          <Eye className="size-4 text-emerald-500" />
          <CardTitle>Global Privacy Control detecte</CardTitle>
        </div>
        <CardDescription>
          Votre navigateur a signale via le signal GPC (Sec-GPC) que vous ne souhaitez pas que vos
          donnees personnelles soient vendues ou partagees. Ce choix a ete enregistre
          automatiquement.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <p className="text-xs text-muted-foreground">
          Le Global Privacy Control (GPC) est un signal envoye par votre navigateur pour exercer
          votre droit de refus de vente de donnees (opt-out). Conformement au CCPA et aux
          legislations applicables, nvbes respecte ce signal et ne vend ni ne partage vos donnees a
          des tiers.
        </p>
      </CardContent>
    </Card>
  );
}
