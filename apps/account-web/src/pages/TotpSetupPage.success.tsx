import { SecuritySetupSuccess } from '@/components/SecuritySetupPanel';

export function TotpSetupSuccessCard({ onBack }: { onBack: () => void }) {
  return (
    <SecuritySetupSuccess
      title="Configuration réussie"
      description="Votre code d'authentification TOTP est maintenant actif."
      onBack={onBack}
    />
  );
}
