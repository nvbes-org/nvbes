import { Download, ShieldAlert } from 'lucide-react';

import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { ErrorMessage, SuccessMessage } from './AccountPrivacyPage.feedback';

export function PrivacyRightsCard({
  exporting,
  exportSuccess,
  exportError,
  onExport,
  onOpenDelete,
}: {
  exporting: boolean;
  exportSuccess: boolean;
  exportError: string | null;
  onExport: () => void;
  onOpenDelete: () => void;
}) {
  return (
    <Card className="animate-fade-slide-up [animation-delay:100ms]">
      <CardHeader>
        <CardTitle>Vos droits RGPD</CardTitle>
        <CardDescription>
          Conformement au reglement general sur la protection des donnees, vous disposez des droits
          suivants.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        <div className="flex items-center justify-between gap-3 py-1">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
              <Download className="size-4 text-muted-foreground" />
            </div>
            <div className="flex min-w-0 flex-col">
              <span className="text-sm font-medium">Exporter mes donnees</span>
              <span className="text-xs text-muted-foreground">
                Recevez une copie de vos donnees personnelles au format JSON.
              </span>
            </div>
          </div>
          <Button
            variant="outline"
            size="sm"
            className="shrink-0"
            onClick={onExport}
            disabled={exporting}
          >
            {exporting ? 'Export...' : 'Exporter'}
          </Button>
        </div>
        {exportSuccess && (
          <SuccessMessage message="Votre demande d'export a ete enregistree. Vous recevrez vos donnees par email dans un delai de 30 jours." />
        )}
        {exportError && <ErrorMessage message={exportError} />}

        <Separator className="my-1" />

        <div className="flex items-center justify-between gap-3 py-1">
          <div className="flex min-w-0 items-center gap-3">
            <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
              <ShieldAlert className="size-4 text-destructive/70" />
            </div>
            <div className="flex min-w-0 flex-col">
              <span className="text-sm font-medium">Supprimer mon compte</span>
              <span className="text-xs text-muted-foreground">
                Supprimez definitivement votre compte et toutes vos donnees.
              </span>
            </div>
          </div>
          <Button
            variant="outline"
            size="sm"
            className="shrink-0 border-destructive/30 text-destructive hover:bg-destructive/10"
            onClick={onOpenDelete}
          >
            Supprimer
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
