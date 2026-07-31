import { SecuritySetupForm } from '@/components/SecuritySetupForm';

export function TotpSetupCard({
  label,
  error,
  loading,
  onLabelChange,
  onCancel,
  onSubmit,
}: {
  label: string;
  error: string | null;
  loading: boolean;
  onLabelChange: (value: string) => void;
  onCancel: () => void;
  onSubmit: () => void;
}) {
  return (
    <SecuritySetupForm
      title="Configurer TOTP"
      fieldId="totp-label"
      fieldLabel="Nom (optionnel)"
      fieldPlaceholder="Ex: Mon téléphone"
      value={label}
      error={error}
      loading={loading}
      submitLabel="Générer le QR code"
      pendingLabel="Génération..."
      onValueChange={onLabelChange}
      onCancel={onCancel}
      onSubmit={onSubmit}
    />
  );
}
