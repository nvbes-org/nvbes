import { useEffect } from 'react';
import { verifyEmailToken } from '../identity.email-verification';

export function useVerifyEmailPageEffects({
  token,
  verified,
  setVerified,
  setStatus,
  setMessage,
  setNow,
}: {
  token: string | null;
  verified: boolean;
  setVerified: React.Dispatch<React.SetStateAction<boolean>>;
  setStatus: React.Dispatch<
    React.SetStateAction<'pending' | 'verifying' | 'sent' | 'verified' | 'error'>
  >;
  setMessage: React.Dispatch<React.SetStateAction<string | null>>;
  setNow: React.Dispatch<React.SetStateAction<Date>>;
}) {
  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(new Date());
    }, 1000);

    return () => window.clearInterval(timer);
  }, [setNow]);

  useEffect(() => {
    if (!token || verified) {
      return;
    }

    let cancelled = false;

    const run = async () => {
      try {
        const result = await verifyEmailToken(token);
        if (cancelled) {
          return;
        }

        if (result.success) {
          setStatus('verified');
          setMessage('Compte vérifié.');
          setVerified(true);
        } else {
          setStatus('error');
          setMessage('La vérification a échoué.');
        }
      } catch (error) {
        if (!cancelled) {
          setStatus('error');
          setMessage(error instanceof Error ? error.message : 'Échec de la vérification.');
        }
      }
    };

    void run();

    return () => {
      cancelled = true;
    };
  }, [token, verified, setMessage, setStatus, setVerified]);
}
