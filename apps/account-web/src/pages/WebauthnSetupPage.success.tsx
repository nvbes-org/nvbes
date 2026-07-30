import { SecuritySetupSuccess } from '@/components/SecuritySetupPanel';

export function WebauthnSetupSuccessCard({
  successText,
  onBack,
}: {
  successText: string;
  onBack: () => void;
}) {
  return <SecuritySetupSuccess title="Clé enregistrée" description={successText} onBack={onBack} />;
}
