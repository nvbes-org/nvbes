import { Button } from '@/components/ui/button';

export function TotpSetupSuccessCard({ onBack }: { onBack: () => void }) {
  return (
    <div className="mx-auto w-full max-w-md space-y-6 text-center">
      <h1 className="text-2xl font-bold">Configuration réussie</h1>
      <p className="text-muted-foreground">
        Votre code d&apos;authentification TOTP est maintenant actif.
      </p>
      <Button onClick={onBack}>Retour à la sécurité</Button>
    </div>
  );
}
