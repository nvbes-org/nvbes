import { identityClient } from '@nvbes/identity-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState, type SubmitEvent } from 'react';
import { identityQueryKeys } from '@/identity.queries';

export function useIdentityEmailAddresses() {
  const queryClient = useQueryClient();
  const [emailDraft, setEmailDraft] = useState('');
  const [error, setError] = useState<string | null>(null);
  const emailsQueryKey = identityQueryKeys.emails;

  const query = useQuery({
    queryKey: emailsQueryKey,
    queryFn: async ({ signal }) => {
      const emails: import('@nvbes/identity-client').EmailAddress[] = [];
      let cursor: string | undefined;
      let primaryMinAgeHours = 0;
      do {
        const page = await identityClient.listEmailsPage({ limit: 200, cursor, signal });
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
    await queryClient.invalidateQueries({ queryKey: emailsQueryKey });
  };

  const addMutation = useMutation({
    mutationFn: (email: string) => identityClient.addSecondaryEmail(email),
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
    mutationFn: (emailId: string) => identityClient.promoteSecondaryEmail(emailId),
    onSuccess: async () => {
      setError(null);
      await invalidate();
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : 'Impossible de promouvoir cet email.');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (emailId: string) => identityClient.deleteSecondaryEmail(emailId),
    onSuccess: async () => {
      setError(null);
      await invalidate();
    },
    onError: (err) => {
      setError(err instanceof Error ? err.message : 'Impossible de supprimer cet email.');
    },
  });

  const resendVerificationMutation = useMutation({
    mutationFn: (emailId: string) => identityClient.resendSecondaryEmailVerification(emailId),
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
