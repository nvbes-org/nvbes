import { accountClient } from '@nvbes/identity-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState, type SubmitEvent } from 'react';
import { accountQueryKeys } from '@/account.queries';
import { useAuthuser } from '@/hooks/useAuthuser';
import { getAccountPersonalInfoQueryKey } from './useAccountPersonalInfoPage.shared';

export function useAccountEmailAddresses() {
  const authuser = useAuthuser();
  const queryClient = useQueryClient();
  const [emailDraft, setEmailDraft] = useState('');
  const [error, setError] = useState<string | null>(null);
  const accountEmailsQueryKey = accountQueryKeys.emails(authuser);

  const query = useQuery({
    queryKey: accountEmailsQueryKey,
    queryFn: async ({ signal }) => {
      const emails: import('@nvbes/identity-client').EmailAddress[] = [];
      let cursor: string | undefined;
      let primaryMinAgeHours = 0;
      do {
        const page = await accountClient.listEmailsPage({ limit: 200, cursor, signal });
        emails.push(...page.emails);
        primaryMinAgeHours = page.primary_min_age_hours;
        cursor = page.has_more && page.next_cursor ? page.next_cursor : undefined;
      } while (cursor);
      return {
        emails,
        primary_min_age_hours: primaryMinAgeHours,
        next_cursor: null,
        has_more: false,
      };
    },
    staleTime: 60 * 1000,
  });

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: accountEmailsQueryKey }),
      queryClient.invalidateQueries({ queryKey: getAccountPersonalInfoQueryKey(authuser) }),
    ]);
  };

  const addMutation = useMutation({
    mutationFn: (email: string) => accountClient.addSecondaryEmail(email),
    onSuccess: async () => {
      setEmailDraft('');
      setError(null);
      await invalidate();
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : "Impossible d'ajouter cet email.");
    },
  });

  const promoteMutation = useMutation({
    mutationFn: (emailId: string) => accountClient.promoteSecondaryEmail(emailId),
    onSuccess: async () => {
      setError(null);
      await invalidate();
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : 'Impossible de promouvoir cet email.');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (emailId: string) => accountClient.deleteSecondaryEmail(emailId),
    onSuccess: async () => {
      setError(null);
      await invalidate();
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : 'Impossible de supprimer cet email.');
    },
  });

  const resendVerificationMutation = useMutation({
    mutationFn: (emailId: string) => accountClient.resendSecondaryEmailVerification(emailId),
    onSuccess: async () => {
      setError(null);
      await invalidate();
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : 'Impossible de renvoyer la verification.');
    },
  });

  const handleAdd = (event: SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    const email = emailDraft.trim();
    if (!email) return;
    setError(null);
    addMutation.mutate(email);
  };

  return {
    adding: addMutation.isPending,
    deletingEmailId: deleteMutation.isPending ? deleteMutation.variables : null,
    emailDraft,
    emails: query.data?.emails ?? [],
    error: error ?? (query.error instanceof Error ? query.error.message : null),
    promotingEmailId: promoteMutation.isPending ? promoteMutation.variables : null,
    primaryMinAgeHours: query.data?.primary_min_age_hours ?? 24,
    resendingEmailId: resendVerificationMutation.isPending
      ? resendVerificationMutation.variables
      : null,
    handleAdd,
    handleDelete: deleteMutation.mutate,
    handlePromote: promoteMutation.mutate,
    handleResendVerification: resendVerificationMutation.mutate,
    setEmailDraft,
  };
}
