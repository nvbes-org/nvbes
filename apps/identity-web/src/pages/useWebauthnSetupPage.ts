import {
  getWebAuthnSupport,
  registerWebAuthnCredential,
  WebauthnBrowserError,
} from '@nvbes/identity-sdk-web';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useEffect, useRef, useState } from 'react';
import { authuserSearch, readAuthuser } from '@/identity.authuser';
import { setupCopy, type WebauthnSetupKind } from './WebauthnSetupPage.shared';

interface WebauthnSupportState {
  supported: boolean;
  platformAuthenticatorAvailable: boolean | null;
  message: string | null;
}

const WEBAUTHN_TIMEOUT_MS = 60_000;

export function useWebauthnSetupPage(kind: WebauthnSetupKind) {
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr);
  const navigate = useNavigate();
  const copy = setupCopy[kind];
  const [step, setStep] = useState<'stepup' | 'register'>('stepup');
  const [label, setLabel] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [support, setSupport] = useState<WebauthnSupportState | null>(null);
  const registrationInFlight = useRef(false);

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
        if (cancelled) {
          return;
        }

        setSupport({
          supported: false,
          platformAuthenticatorAvailable: null,
          message: copy.unsupportedText,
        });
      });

    return () => {
      cancelled = true;
    };
  }, [copy.browserText, copy.unsupportedText]);

  const register = async () => {
    if (registrationInFlight.current) {
      return;
    }

    registrationInFlight.current = true;
    setLoading(true);
    setError(null);

    try {
      await registerWebAuthnCredential('', label || undefined, kind, undefined, {
        timeoutMs: WEBAUTHN_TIMEOUT_MS,
      });
      void navigate({ to: '/account/mfa/recovery-codes', search: authuserSearch(authuser) });
    } catch (err) {
      if (err instanceof Error && err.message.includes('step_up_required')) {
        setStep('stepup');
      } else if (err instanceof WebauthnBrowserError && err.code === 'webauthn_timeout') {
        setError(copy.timeoutText);
      } else {
        setError(err instanceof Error ? err.message : 'WebAuthn registration failed');
      }
    } finally {
      registrationInFlight.current = false;
      setLoading(false);
    }
  };

  return {
    copy,
    step,
    label,
    error,
    loading,
    support,
    showPlatformWarning: kind === 'passkey' && support?.platformAuthenticatorAvailable === false,
    navigateBack: () =>
      void navigate({ to: '/account/security', search: authuserSearch(authuser) }),
    onStepUpSuccess: () => setStep('register'),
    onLabelChange: setLabel,
    onRegister: register,
    supported: support?.supported ?? null,
  };
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
