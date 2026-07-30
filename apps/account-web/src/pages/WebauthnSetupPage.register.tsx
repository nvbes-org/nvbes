import { FeedbackAlert } from '@/components/FeedbackAlert';
import { SecuritySetupForm } from '@/components/SecuritySetupForm';

export function WebauthnSetupRegisterCard({
  title,
  label,
  labelPlaceholder,
  supportMessage,
  supported,
  showPlatformWarning,
  error,
  loading,
  buttonText,
  onLabelChange,
  onCancel,
  onSubmit,
}: {
  title: string;
  label: string;
  labelPlaceholder: string;
  supportMessage: string | null;
  supported: boolean | null;
  showPlatformWarning: boolean;
  error: string | null;
  loading: boolean;
  buttonText: string;
  onLabelChange: (value: string) => void;
  onCancel: () => void;
  onSubmit: () => void;
}) {
  return (
    <SecuritySetupForm
      title={title}
      fieldId="webauthn-label"
      fieldLabel="Nom de la clé"
      fieldPlaceholder={labelPlaceholder}
      value={label}
      error={error}
      loading={loading}
      submitLabel={buttonText}
      pendingLabel="Enregistrement..."
      submitDisabled={supported === null || supported === false}
      onValueChange={onLabelChange}
      onCancel={onCancel}
      onSubmit={onSubmit}
    >
      {supportMessage ? (
        <p className={supported ? 'text-sm text-muted-foreground' : 'text-sm text-destructive'}>
          {supportMessage}
        </p>
      ) : null}
      {showPlatformWarning ? (
        <FeedbackAlert tone="error">
          Aucun authenticator local compatible biométrie/Touch ID n’a été détecté.
        </FeedbackAlert>
      ) : null}
    </SecuritySetupForm>
  );
}
