import { Download, ShieldAlert } from 'lucide-react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Spinner } from '@/components/ui/spinner';
import { ErrorMessage, SuccessMessage } from './AccountPrivacyPage.feedback';

export function PrivacyExportCard({
  exporting,
  exportSuccess,
  exportError,
  onExport,
}: {
  exporting: boolean;
  exportSuccess: boolean;
  exportError: string | null;
  onExport: () => void;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>
          <h3>Récupérer vos données</h3>
        </CardTitle>
        <CardDescription>
          Demandez une copie portable des données personnelles associées à votre compte.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {exportSuccess && (
          <SuccessMessage message="Votre demande est enregistrée. Vous recevrez vos données par e-mail dès que l’export sera prêt." />
        )}
        {exportError && <ErrorMessage message={exportError} />}
      </CardContent>
      <CardFooter className="justify-start">
        <Button onClick={onExport} disabled={exporting}>
          {exporting ? <Spinner data-icon="inline-start" /> : <Download data-icon="inline-start" />}
          {exporting ? 'Préparation…' : 'Préparer mon export'}
        </Button>
      </CardFooter>
    </Card>
  );
}

export function PrivacyDeleteCard({ onOpenDelete }: { onOpenDelete: () => void }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>
          <h3>Supprimer votre compte</h3>
        </CardTitle>
        <CardDescription>
          Efface définitivement votre compte et lance la suppression des données associées.
        </CardDescription>
        <CardAction>
          <Badge variant="destructive">Irréversible</Badge>
        </CardAction>
      </CardHeader>
      <CardContent>
        <p className="text-sm text-muted-foreground">
          Les obligations légales de conservation peuvent imposer de garder certaines traces
          limitées pendant leur durée réglementaire.
        </p>
      </CardContent>
      <CardFooter>
        <Button variant="destructive" onClick={onOpenDelete}>
          <ShieldAlert data-icon="inline-start" />
          Commencer la suppression
        </Button>
      </CardFooter>
    </Card>
  );
}
