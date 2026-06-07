import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Separator } from '@/components/ui/separator';

export function TotpConfirmCard({
  qrUrl,
  secretBase32,
  totpCode,
  error,
  loading,
  onCopySecret,
  onTotpCodeChange,
  onCancel,
  onSubmit,
}: {
  qrUrl: string | null;
  secretBase32: string;
  totpCode: string;
  error: string | null;
  loading: boolean;
  onCopySecret: () => void;
  onTotpCodeChange: (value: string) => void;
  onCancel: () => void;
  onSubmit: (event: React.FormEvent<HTMLFormElement>) => void;
}) {
  return (
    <div className="flex min-h-screen items-center justify-center">
      <div className="w-full max-w-md space-y-6">
        <h1 className="text-2xl font-bold">Scanner le QR code</h1>

        {qrUrl ? (
          <div className="flex justify-center">
            <img
              src={qrUrl}
              alt="QR code TOTP"
              className="rounded-lg border"
              width={200}
              height={200}
            />
          </div>
        ) : null}

        <Separator />

        <div className="space-y-2">
          <p className="text-sm text-muted-foreground">
            Ou entrez ce code manuellement dans votre application d&apos;authentification :
          </p>
          <div className="flex items-center gap-2">
            <code className="flex-1 break-all rounded bg-muted px-3 py-2 text-sm font-mono">
              {secretBase32}
            </code>
            <Button variant="outline" size="sm" onClick={onCopySecret}>
              Copier
            </Button>
          </div>
        </div>

        <Separator />

        <form onSubmit={onSubmit} className="space-y-4">
          <div className="space-y-2">
            <label htmlFor="totp-confirm-code" className="text-sm font-medium">
              Code de vérification
            </label>
            <Input
              id="totp-confirm-code"
              type="text"
              inputMode="numeric"
              autoComplete="one-time-code"
              placeholder="000000"
              maxLength={6}
              value={totpCode}
              onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
                onTotpCodeChange(event.target.value)
              }
              required
            />
          </div>
          {error ? <p className="text-sm text-destructive">{error}</p> : null}
          <div className="flex gap-2">
            <Button type="button" variant="outline" className="flex-1" onClick={onCancel}>
              Annuler
            </Button>
            <Button type="submit" className="flex-1" disabled={loading}>
              {loading ? 'Vérification...' : 'Valider'}
            </Button>
          </div>
        </form>
      </div>
    </div>
  );
}
