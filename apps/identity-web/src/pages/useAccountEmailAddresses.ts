import { identityClient } from '@nvbes/identity-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useLocation } from '@tanstack/react-router';
import { useState, type FormEvent } from 'react';
import { accountQueryKeys } from '@/account.queries';
import { readAuthuser } from '@/identity.authuser';
import { getAccountPersonalInfoQueryKey } from './useAccountPersonalInfoPage.shared';

export function useAccountEmailAddresses() {
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr);
  const queryClient = useQueryClient();
  const [emailDraft, setEmailDraft] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const accountEmailsQueryKey = accountQueryKeys.emails(authuser);

  const query = useQuery({
    queryKey: accountEmailsQueryKey,
    queryFn: ({ signal }) => identityClient.listEmails({ signal }),
    staleTime: 60 * 1000,
  });

  const invalidate = async () => {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: accountEmailsQueryKey }),
      queryClient.invalidateQueries({ queryKey: getAccountPersonalInfoQueryKey(authuser) }),
    ]);
  };

  const addMutation = useMutation({
    mutationFn: (email: string) => identityClient.addSecondaryEmail(email),
    onSuccess: async () => {
      setEmailDraft('');
      setError(null);
      setSuccess('Email ajoute. Un lien de verification vient d etre envoye.');
      await invalidate();
    },
    onError: (err) => {
      setSuccess(null);
      setError(err instanceof Error ? err.message : "Impossible d'ajouter cet email.");
    },
  });

  const promoteMutation = useMutation({
    mutationFn: (emailId: string) => identityClient.promoteSecondaryEmail(emailId),
    onSuccess: async () => {
      setError(null);
      setSuccess('Email principal mis a jour.');
      await invalidate();
    },
    onError: (err) => {
      setSuccess(null);
      setError(err instanceof Error ? err.message : 'Impossible de promouvoir cet email.');
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (emailId: string) => identityClient.deleteSecondaryEmail(emailId),
    onSuccess: async () => {
      setError(null);
      setSuccess('Email secondaire supprime.');
      await invalidate();
    },
    onError: (err) => {
      setSuccess(null);
      setError(err instanceof Error ? err.message : 'Impossible de supprimer cet email.');
    },
  });

  const resendVerificationMutation = useMutation({
    mutationFn: (emailId: string) => identityClient.resendSecondaryEmailVerification(emailId),
    onSuccess: async () => {
      setError(null);
      setSuccess('Lien de verification renvoye.');
      await invalidate();
    },
    onError: (err) => {
      setSuccess(null);
      setError(err instanceof Error ? err.message : 'Impossible de renvoyer la verification.');
    },
  });

  const handleAdd = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const email = emailDraft.trim();
    if (!email) return;
    setError(null);
    setSuccess(null);
    addMutation.mutate(email);
  };

  return {
    emailDraft,
    emails: query.data?.emails ?? [],
    error: error ?? (query.error instanceof Error ? query.error.message : null),
    loading:
      query.isLoading ||
      addMutation.isPending ||
      promoteMutation.isPending ||
      deleteMutation.isPending ||
      resendVerificationMutation.isPending,
    primaryMinAgeHours: query.data?.primary_min_age_hours ?? 24,
    success,
    handleAdd,
    handleDelete: deleteMutation.mutate,
    handlePromote: promoteMutation.mutate,
    handleResendVerification: resendVerificationMutation.mutate,
    setEmailDraft,
  };
}
