import { type GpcStatus, identityClient, type UserConsent } from '@nvbes/identity-client';
import { useEffect } from 'react';

export function useAccountPrivacyPageLoad({
  setConsents,
  setGpc,
  setLoading,
}: {
  setConsents: React.Dispatch<React.SetStateAction<UserConsent[]>>;
  setGpc: React.Dispatch<React.SetStateAction<GpcStatus | null>>;
  setLoading: React.Dispatch<React.SetStateAction<boolean>>;
}) {
  useEffect(() => {
    const fetchConsents = async () => {
      try {
        const allConsents = await identityClient.listConsents();
        setConsents(allConsents.filter((consent) => consent.revoked_at === null));
      } finally {
        setLoading(false);
      }
    };

    const fetchGpcStatus = async () => {
      try {
        setGpc(await identityClient.gpcStatus());
      } catch {
        // GPC endpoint is best-effort; silently ignore failures
      }
    };

    void fetchConsents();
    void fetchGpcStatus();
  }, [setConsents, setGpc, setLoading]);
}
