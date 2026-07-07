import { Button } from '@/components/ui/button';

export function WebauthnSetupSuccessCard({
  successText,
  onBack,
}: {
  successText: string;
  onBack: () => void;
}) {
  return (
    <div className="flex min-h-screen items-center justify-center">
      <div className="w-full max-w-md space-y-6 text-center">
        <h1 className="text-2xl font-bold">Clé enregistrée</h1>
        <p className="text-muted-foreground">{successText}</p>
        <Button onClick={onBack}>Retour à la sécurité</Button>
      </div>
    </div>
  );
}
