import { accountClient, type GpcStatus, type UserConsent } from '@nvbes/identity-client';
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
        const allConsents: UserConsent[] = [];
        let cursor: string | undefined;
        do {
          const page = await accountClient.listConsentsPage({ limit: 200, cursor });
          allConsents.push(...page.consents);
          cursor = page.has_more && page.next_cursor ? page.next_cursor : undefined;
        } while (cursor);
        setConsents(allConsents.filter((consent) => consent.revoked_at === null));
      } finally {
        setLoading(false);
      }
    };

    const fetchGpcStatus = async () => {
      try {
        setGpc(await accountClient.gpcStatus());
      } catch {
        // GPC endpoint is best-effort; silently ignore failures
      }
    };

    void fetchConsents();
    void fetchGpcStatus();
  }, [setConsents, setGpc, setLoading]);
}
