import { HttpError } from '@nvbes/http-client';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useEffect, useState } from 'react';

import type { VerifyEmailApiError, VerifyEmailResultState } from './VerifyEmailResultPage.shared';
import { verifyEmailToken } from '../identity.email-verification';

function errorBody(error: unknown): VerifyEmailApiError {
  if (error instanceof HttpError && typeof error.body === 'object' && error.body !== null) {
    return error.body as VerifyEmailApiError;
  }

  return {};
}

export function useVerifyEmailResultPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const searchParams = new URLSearchParams(location.searchStr);
  const token = searchParams.get('token');
  const [result, setResult] = useState<VerifyEmailResultState>({
    kind: 'verifying',
  });

  useEffect(() => {
    if (!token) {
      setResult({ kind: 'token_invalid' });
      return;
    }

    let cancelled = false;

    const verify = async () => {
      try {
        const data = await verifyEmailToken(token);
        if (cancelled) {
          return;
        }

        if (data.success) {
          setResult({ kind: 'success', email: data.user?.email ?? '' });
          return;
        }

        setResult({ kind: 'error', message: 'Échec de la vérification.' });
      } catch (error) {
        if (cancelled) {
          return;
        }

        const body = errorBody(error);
        const code = body.error?.code ?? '';
        const message = body.error?.message ?? '';

        if (code === 'verification_token_expired' || message.toLowerCase().includes('expired')) {
          setResult({ kind: 'token_expired' });
        } else if (
          code === 'verification_token_not_found' ||
          message.toLowerCase().includes('invalid')
        ) {
          setResult({ kind: 'token_invalid' });
        } else {
          setResult({
            kind: 'error',
            message: message || 'Échec de la vérification.',
          });
        }
      }
    };

    void verify();

    return () => {
      cancelled = true;
    };
  }, [token]);

  return {
    navigate,
    result,
  };
}
