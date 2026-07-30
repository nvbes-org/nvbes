import { ClipboardButton } from '@nvbes/web-ui';
import { FeedbackAlert } from '@/components/FeedbackAlert';
import { Button } from '@/components/ui/button';
import { SecuritySetupActions, SecuritySetupPanel } from '@/components/SecuritySetupPanel';

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
    <SecuritySetupPanel
      title="Vos codes de récupération"
      className="space-y-6"
      titleClassName="text-3xl"
    >
      <FeedbackAlert tone="error">
        Conservez ces codes dans un endroit sûr. Ils ne seront plus affichés. Chaque code ne peut
        être utilisé qu&apos;une seule fois.
      </FeedbackAlert>

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

      <SecuritySetupActions>
        <ClipboardButton value={codes.join('\n')} label="Copier" className="h-10 flex-1" />
        <Button variant="outline" className="flex-1" onClick={onDownload}>
          Télécharger
        </Button>
      </SecuritySetupActions>

      <Button className="w-full" onClick={onBack}>
        Retour à la sécurité
      </Button>
    </SecuritySetupPanel>
  );
}
