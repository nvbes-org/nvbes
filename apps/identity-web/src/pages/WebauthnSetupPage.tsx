import StepUpForm from '@/components/StepUpForm';
import {
  getWebAuthnSupport,
  registerWebAuthnCredential,
  WebauthnBrowserError,
} from '@nvbes/identity-sdk-web';
import { useNavigate } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

type WebauthnSetupKind = 'passkey' | 'security_key';

const setupCopy: Record<
  WebauthnSetupKind,
  {
    title: string;
    labelPlaceholder: string;
    stepUpText: string;
    successText: string;
    timeoutText: string;
    buttonText: string;
    unsupportedText: string;
    browserText: string;
  }
> = {
  passkey: {
    title: 'Enregistrer une passkey',
    labelPlaceholder: 'Ex: Touch ID du Mac',
    stepUpText: 'Pour enregistrer une passkey, veuillez confirmer votre identité.',
    successText: 'Votre passkey est maintenant active.',
    timeoutText: 'Passkey registration timed out. Please try again and confirm with Touch ID.',
    buttonText: 'Enregistrer la passkey',
    unsupportedText:
      'Les passkeys ne sont pas disponibles dans ce navigateur. Ouvrez cette page dans Chrome, Arc, Safari ou Edge.',
    browserText:
      'Si aucune fenêtre Touch ID ne s’ouvre, quittez le navigateur intégré VS Code/Electron et ouvrez cette page dans Chrome, Arc, Safari ou Edge.',
  },
  security_key: {
    title: 'Enregistrer une clé de sécurité',
    labelPlaceholder: 'Ex: YubiKey bleue',
    stepUpText: 'Pour enregistrer une clé de sécurité, veuillez confirmer votre identité.',
    successText: 'Votre clé de sécurité est maintenant active.',
    timeoutText: 'Security key registration timed out. Please try again and touch your key.',
    buttonText: 'Enregistrer la clé',
    unsupportedText:
      'Les clés de sécurité WebAuthn ne sont pas disponibles dans ce navigateur. Ouvrez cette page dans Chrome, Arc, Safari ou Edge.',
    browserText:
      'Insérez votre clé avant de continuer. Si aucune demande ne s’affiche, utilisez Chrome, Arc, Safari ou Edge hors du navigateur intégré VS Code/Electron.',
  },
};

interface WebauthnSupportState {
  supported: boolean;
  platformAuthenticatorAvailable: boolean | null;
  message: string | null;
}

export default function WebauthnSetupPage({ kind = 'security_key' }: { kind?: WebauthnSetupKind }) {
  const navigate = useNavigate();
  const copy = setupCopy[kind];

  const [step, setStep] = useState<'stepup' | 'register' | 'done'>('stepup');
  const [label, setLabel] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [support, setSupport] = useState<WebauthnSupportState | null>(null);

  const webauthnTimeoutMs = 60_000;

  useEffect(() => {
    let cancelled = false;

    getWebAuthnSupport()
      .then((report) => {
        if (cancelled) {
          return;
        }

        setSupport({
          supported: report.supported,
          platformAuthenticatorAvailable: report.platform_authenticator_available,
          message: webauthnSupportMessage(report, copy.unsupportedText, copy.browserText),
        });
      })
      .catch(() => {
        if (!cancelled) {
          setSupport({
            supported: false,
            platformAuthenticatorAvailable: null,
            message: copy.unsupportedText,
          });
        }
      });

    return () => {
      cancelled = true;
    };
  }, [copy.browserText, copy.unsupportedText]);

  const handleRegister = async () => {
    setLoading(true);
    setError(null);
    try {
      await registerWebAuthnCredential('', label || undefined, kind, undefined, {
        timeoutMs: webauthnTimeoutMs,
      });
      setStep('done');
    } catch (err) {
      if (err instanceof Error && err.message.includes('step_up_required')) {
        setStep('stepup');
      } else if (err instanceof WebauthnBrowserError && err.code === 'webauthn_timeout') {
        setError(copy.timeoutText);
      } else {
        setError(err instanceof Error ? err.message : 'WebAuthn registration failed');
      }
    } finally {
      setLoading(false);
    }
  };

  if (step === 'stepup') {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <StepUpForm
          onSuccess={() => setStep('register')}
          onCancel={() => void navigate({ to: '/account/security' })}
          description={copy.stepUpText}
        />
      </div>
    );
  }

  if (step === 'register') {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <div className="w-full max-w-md space-y-4">
          <h1 className="text-2xl font-bold">{copy.title}</h1>
          <div className="space-y-2">
            <label htmlFor="webauthn-label" className="text-sm font-medium">
              Nom de la clé
            </label>
            <Input
              id="webauthn-label"
              placeholder={copy.labelPlaceholder}
              value={label}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setLabel(e.target.value)}
            />
          </div>
          {support?.message && (
            <p
              className={
                support.supported ? 'text-sm text-muted-foreground' : 'text-sm text-destructive'
              }
            >
              {support.message}
            </p>
          )}
          {kind === 'passkey' && support?.platformAuthenticatorAvailable === false && (
            <p className="text-sm text-destructive">
              Aucun authenticator local compatible passkey/Touch ID n’a été détecté.
            </p>
          )}
          {error && <p className="text-sm text-destructive">{error}</p>}
          <div className="flex gap-2">
            <Button
              variant="outline"
              className="flex-1"
              onClick={() => void navigate({ to: '/account/security' })}
            >
              Annuler
            </Button>
            <Button
              className="flex-1"
              disabled={loading || support === null || support.supported === false}
              onClick={handleRegister}
            >
              {loading ? 'Enregistrement...' : copy.buttonText}
            </Button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-screen items-center justify-center">
      <div className="w-full max-w-md text-center space-y-6">
        <h1 className="text-2xl font-bold">Clé enregistrée</h1>
        <p className="text-muted-foreground">{copy.successText}</p>
        <Button onClick={() => void navigate({ to: '/account/security' })}>
          Retour à la sécurité
        </Button>
      </div>
    </div>
  );
}

function webauthnSupportMessage(
  report: Awaited<ReturnType<typeof getWebAuthnSupport>>,
  unsupportedText: string,
  browserText: string,
): string | null {
  if (report.is_likely_embedded_runtime) {
    return browserText;
  }

  if (!report.supported) {
    return unsupportedText;
  }

  return null;
}
