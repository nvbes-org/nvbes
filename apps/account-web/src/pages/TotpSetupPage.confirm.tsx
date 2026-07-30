import { InvisibleUnicodeWarning } from '@nvbes/web-runtime';
import { ClipboardButton } from '@nvbes/web-ui';
import { FeedbackAlert } from '@/components/FeedbackAlert';
import { Button } from '@/components/ui/button';
import { Card } from '@/components/ui/card';
import { QRCode } from '@/components/kibo-ui/qr-code';
import { Field, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import { Separator } from '@/components/ui/separator';

export function TotpConfirmCard({
  qrData,
  secretBase32,
  totpCode,
  error,
  loading,
  onTotpCodeChange,
  onCancel,
  onSubmit,
}: {
  qrData: string | null;
  secretBase32: string;
  totpCode: string;
  error: string | null;
  loading: boolean;
  onTotpCodeChange: (value: string) => void;
  onCancel: () => void;
  onSubmit: (event: React.SubmitEvent<HTMLFormElement>) => void;
}) {
  return (
    <div className="mx-auto w-full max-w-md space-y-6">
      <h1 className="text-2xl font-bold">Scanner le QR code</h1>

      {qrData ? (
        <div className="flex justify-center">
          <Card className="bg-white p-2 border">
            <QRCode
              data={qrData}
              aria-label="QR code TOTP"
              className="size-[200px]"
              foreground="#000000"
              background="#ffffff"
              robustness="H"
            />
          </Card>
        </div>
      ) : null}

      <Separator />

      <div className="space-y-2">
        <p className="text-sm text-muted-foreground">
          Ou entrez ce code manuellement dans votre application d&apos;authentification :
        </p>
        <div className="flex items-center gap-2">
          <code className="flex-1 break-all rounded border bg-white px-3 py-2 text-sm font-mono">
            {secretBase32}
          </code>
          <ClipboardButton value={secretBase32} label="Copier" className="h-9" />
        </div>
        <InvisibleUnicodeWarning value={secretBase32} />
      </div>

      <Separator />

      <form onSubmit={onSubmit} className="space-y-4">
        <Field>
          <FieldLabel htmlFor="totp-confirm-code">Code de vérification</FieldLabel>
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
        </Field>
        {error ? <FeedbackAlert tone="error">{error}</FeedbackAlert> : null}
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
  );
}
