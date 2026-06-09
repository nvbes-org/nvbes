import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

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
    <div className="flex min-h-screen items-center justify-center">
      <div className="w-full max-w-md space-y-4">
        <h1 className="text-2xl font-bold">Configurer TOTP</h1>
        <div className="space-y-2">
          <Label htmlFor="totp-label">
            Nom (optionnel)
          </Label>
          <Input
            id="totp-label"
            placeholder="Ex: Mon téléphone"
            value={label}
            onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
              onLabelChange(event.target.value)
            }
          />
        </div>
        {error ? (
          <Alert variant="destructive">
            <AlertDescription>{error}</AlertDescription>
          </Alert>
        ) : null}
        <div className="flex gap-2">
          <Button variant="outline" className="flex-1" onClick={onCancel}>
            Annuler
          </Button>
          <Button className="flex-1" disabled={loading} onClick={onSubmit}>
            {loading ? 'Génération...' : 'Générer le QR code'}
          </Button>
        </div>
      </div>
    </div>
  );
}
