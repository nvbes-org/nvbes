import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

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
    <div className="flex min-h-screen items-center justify-center">
      <div className="w-full max-w-md space-y-4">
        <h1 className="text-2xl font-bold">{title}</h1>
        <div className="space-y-2">
          <label htmlFor="webauthn-label" className="text-sm font-medium">
            Nom de la clé
          </label>
          <Input
            id="webauthn-label"
            placeholder={labelPlaceholder}
            value={label}
            onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
              onLabelChange(event.target.value)
            }
          />
        </div>
        {supportMessage ? (
          <p className={supported ? 'text-sm text-muted-foreground' : 'text-sm text-destructive'}>
            {supportMessage}
          </p>
        ) : null}
        {showPlatformWarning ? (
          <p className="text-sm text-destructive">
            Aucun authenticator local compatible passkey/Touch ID n’a été détecté.
          </p>
        ) : null}
        {error ? <p className="text-sm text-destructive">{error}</p> : null}
        <div className="flex gap-2">
          <Button variant="outline" className="flex-1" onClick={onCancel}>
            Annuler
          </Button>
          <Button
            className="flex-1"
            disabled={loading || supported === null || supported === false}
            onClick={onSubmit}
          >
            {loading ? 'Enregistrement...' : buttonText}
          </Button>
        </div>
      </div>
    </div>
  );
}
