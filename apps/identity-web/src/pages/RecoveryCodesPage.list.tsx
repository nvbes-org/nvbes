import { Button } from '@/components/ui/button';

export function RecoveryCodesListStep({
  codes,
  copied,
  onBack,
  onCopy,
  onDownload,
}: {
  codes: string[];
  copied: boolean;
  onBack: () => void;
  onCopy: () => void;
  onDownload: () => void;
}) {
  return (
    <div className="flex min-h-screen items-center justify-center">
      <div className="w-full max-w-md space-y-6">
        <h1 className="text-2xl font-bold">Vos codes de récupération</h1>

        <div className="rounded-lg bg-destructive/10 p-4 text-sm text-destructive">
          Conservez ces codes dans un endroit sûr. Ils ne seront plus affichés. Chaque code ne peut
          être utilisé qu&apos;une seule fois.
        </div>

        <div className="grid grid-cols-2 gap-2">
          {codes.map((code, index) => (
            <code
              key={`${code}-${index}`}
              className="rounded bg-muted px-3 py-2 text-center text-sm font-mono"
            >
              {code}
            </code>
          ))}
        </div>

        <div className="flex gap-2">
          <Button variant="outline" className="flex-1" onClick={onCopy}>
            {copied ? 'Copié !' : 'Copier'}
          </Button>
          <Button variant="outline" className="flex-1" onClick={onDownload}>
            Télécharger
          </Button>
        </div>

        <Button className="w-full" onClick={onBack}>
          Retour à la sécurité
        </Button>
      </div>
    </div>
  );
}
