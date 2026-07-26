import { ClipboardButton } from '@nvbes/web-ui';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';

export function RecoveryCodesListStep({
  codes,
  onBack,
  onDownload,
}: {
  codes: string[];
  onBack: () => void;
  onDownload: () => void;
}) {
  return (
    <div className="mx-auto w-full max-w-md space-y-6">
      <h1 className="text-3xl font-bold">Vos codes de récupération</h1>

      <Alert variant="destructive">
        <AlertDescription>
          Conservez ces codes dans un endroit sûr. Ils ne seront plus affichés. Chaque code ne peut
          être utilisé qu&apos;une seule fois.
        </AlertDescription>
      </Alert>

      <div className="grid grid-cols-2 gap-2">
        {codes.map((code, index) => (
          <code
            key={`${code}-${index}`}
            className="rounded border bg-white px-3 py-2 text-center text-sm font-mono"
          >
            {code}
          </code>
        ))}
      </div>

      <div className="flex gap-2">
        <ClipboardButton value={codes.join('\n')} label="Copier" className="h-10 flex-1" />
        <Button variant="outline" className="flex-1" onClick={onDownload}>
          Télécharger
        </Button>
      </div>

      <Button className="w-full" onClick={onBack}>
        Retour à la sécurité
      </Button>
    </div>
  );
}
