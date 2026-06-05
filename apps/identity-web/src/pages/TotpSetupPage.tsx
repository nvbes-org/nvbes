import StepUpForm from '@/components/StepUpForm';
import { confirmTotp, setupTotp } from '@nvbes/identity-sdk-web';
import { useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Separator } from '@/components/ui/separator';

export default function TotpSetupPage() {
  const navigate = useNavigate();

  const [step, setStep] = useState<'stepup' | 'setup' | 'confirm' | 'done'>('stepup');
  const [label, setLabel] = useState('');
  const [factorId, setFactorId] = useState('');
  const [secretBase32, setSecretBase32] = useState('');
  const [provisioningUri, setProvisioningUri] = useState('');
  const [totpCode, setTotpCode] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  const handleSetup = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await setupTotp('', label || undefined);
      setFactorId(result.factor.id);
      setSecretBase32(result.secret_base32);
      setProvisioningUri(result.provisioning_uri);
      setStep('confirm');
    } catch (err) {
      const message = err instanceof Error ? err.message : 'TOTP setup failed';
      if (message.includes('step_up_required')) {
        setStep('stepup');
      } else {
        setError(message);
      }
    } finally {
      setLoading(false);
    }
  };

  const handleConfirm = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      await confirmTotp('', factorId, totpCode);
      setStep('done');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'TOTP confirmation failed');
    } finally {
      setLoading(false);
    }
  };

  const copySecret = () => {
    navigator.clipboard.writeText(secretBase32);
  };

  const qrUrl = provisioningUri
    ? `https://api.qrserver.com/v1/create-qr-code/?size=200x200&data=${encodeURIComponent(provisioningUri)}`
    : null;

  if (step === 'stepup') {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <StepUpForm
          onSuccess={() => setStep('setup')}
          onCancel={() => void navigate({ to: '/account/security' })}
          description="Pour configurer un code d'authentification, veuillez confirmer votre identité."
        />
      </div>
    );
  }

  if (step === 'setup') {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <div className="w-full max-w-md space-y-4">
          <h1 className="text-2xl font-bold">Configurer TOTP</h1>
          <div className="space-y-2">
            <label htmlFor="totp-label" className="text-sm font-medium">
              Nom (optionnel)
            </label>
            <Input
              id="totp-label"
              placeholder="Ex: Mon téléphone"
              value={label}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setLabel(e.target.value)}
            />
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
          <div className="flex gap-2">
            <Button
              variant="outline"
              className="flex-1"
              onClick={() => void navigate({ to: '/account/security' })}
            >
              Annuler
            </Button>
            <Button className="flex-1" disabled={loading} onClick={handleSetup}>
              {loading ? 'Génération...' : 'Générer le QR code'}
            </Button>
          </div>
        </div>
      </div>
    );
  }

  if (step === 'confirm') {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <div className="w-full max-w-md space-y-6">
          <h1 className="text-2xl font-bold">Scanner le QR code</h1>

          {qrUrl && (
            <div className="flex justify-center">
              <img
                src={qrUrl}
                alt="QR code TOTP"
                className="rounded-lg border"
                width={200}
                height={200}
              />
            </div>
          )}

          <Separator />

          <div className="space-y-2">
            <p className="text-sm text-muted-foreground">
              Ou entrez ce code manuellement dans votre application d'authentification :
            </p>
            <div className="flex items-center gap-2">
              <code className="flex-1 rounded bg-muted px-3 py-2 text-sm font-mono break-all">
                {secretBase32}
              </code>
              <Button variant="outline" size="sm" onClick={copySecret}>
                Copier
              </Button>
            </div>
          </div>

          <Separator />

          <form onSubmit={handleConfirm} className="space-y-4">
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
                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setTotpCode(e.target.value)}
                required
              />
            </div>
            {error && <p className="text-sm text-destructive">{error}</p>}
            <div className="flex gap-2">
              <Button
                type="button"
                variant="outline"
                className="flex-1"
                onClick={() => void navigate({ to: '/account/security' })}
              >
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

  return (
    <div className="flex min-h-screen items-center justify-center">
      <div className="w-full max-w-md text-center space-y-6">
        <h1 className="text-2xl font-bold">Configuration réussie</h1>
        <p className="text-muted-foreground">
          Votre code d'authentification TOTP est maintenant actif.
        </p>
        <Button onClick={() => void navigate({ to: '/account/security' })}>
          Retour à la sécurité
        </Button>
      </div>
    </div>
  );
}
