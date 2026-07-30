import { clientErrorMessage } from '@nvbes/web-runtime';
import { storePasswordCredential } from '@nvbes/identity-sdk-web';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useEffect, useMemo, useState } from 'react';
import { useAccountContext } from '@/hooks/useAccountContext';
import { useAuthuser } from '@/hooks/useAuthuser';
import { accountClient } from '@nvbes/identity-client';
import {
  MAX_PASSWORD_LENGTH,
  MIN_PASSWORD_LENGTH,
  passwordHasSupportedLength,
} from '../identity.password.policy';
import { type ChangePasswordInput, changePassword, forgotPassword } from '../identity.password.api';
import { estimatePasswordStrength } from '@/lib/password-strength';

import { isStepUpRequiredError } from '@/identity.step-up';

const RESET_EMAIL_RESEND_DELAY_MS = 60_000;

export function useAccountPasswordPage() {
  const authuser = useAuthuser();
  const { me } = useAccountContext();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const [showStepUp, setShowStepUp] = useState(false);
  const [showSessionPrompt, setShowSessionPrompt] = useState(false);
  const [showSessionStepUp, setShowSessionStepUp] = useState(false);
  const [resetEmailSent, setResetEmailSent] = useState(false);
  const [resetEmailResendAt, setResetEmailResendAt] = useState<number | null>(null);
  const [countdownNow, setCountdownNow] = useState(() => Date.now());

  useEffect(() => {
    if (resetEmailResendAt === null) {
      return;
    }
    const tick = () => {
      const now = Date.now();
      setCountdownNow(now);
      if (now >= resetEmailResendAt) {
        setResetEmailResendAt(null);
      }
    };
    tick();
    const intervalId = window.setInterval(tick, 1_000);
    return () => window.clearInterval(intervalId);
  }, [resetEmailResendAt]);

  const resetEmailResendSeconds =
    resetEmailResendAt === null
      ? 0
      : Math.max(0, Math.ceil((resetEmailResendAt - countdownNow) / 1_000));

  const passwordResult = useMemo(() => {
    if (!newPassword) {
      return null;
    }
    return estimatePasswordStrength(newPassword);
  }, [newPassword]);

  const mutation = useMutation({
    mutationFn: (input: ChangePasswordInput) => changePassword(input),
    onSuccess: async (_result, input) => {
      if (me?.user.email) {
        await storePasswordCredential(me.user.email, input.new_password, me.user.email);
      }
      setSuccess(true);
      setNewPassword('');
      setConfirmPassword('');
      setShowStepUp(false);
      setShowSessionPrompt(true);
      void queryClient.invalidateQueries({ queryKey: ['identity', 'account'] });
    },
    onError: (err) => {
      if (isStepUpRequiredError(err)) {
        setShowStepUp(true);
      } else {
        setError(
          clientErrorMessage(
            err,
            'Impossible de modifier le mot de passe. Veuillez verifier la conformite du nouveau mot de passe.',
          ),
        );
      }
    },
  });

  const resetEmailMutation = useMutation({
    mutationFn: () => forgotPassword(me?.user.email ?? ''),
    onSuccess: () => {
      const now = Date.now();
      setResetEmailSent(true);
      setCountdownNow(now);
      setResetEmailResendAt(now + RESET_EMAIL_RESEND_DELAY_MS);
    },
    onError: (err) =>
      setError(clientErrorMessage(err, "Impossible d'envoyer l'email de réinitialisation.")),
  });

  const handleSubmit = async (event: React.SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setSuccess(false);

    if (!passwordHasSupportedLength(newPassword)) {
      setError(
        `Le nouveau mot de passe doit contenir entre ${MIN_PASSWORD_LENGTH} et ${MAX_PASSWORD_LENGTH} caractères.`,
      );
      return;
    }

    if ((passwordResult?.score ?? 0) < 3) {
      setError('Le mot de passe est trop faible. Veuillez choisir un mot de passe plus fort.');
      return;
    }

    if (newPassword !== confirmPassword) {
      setError('Les mots de passe ne correspondent pas.');
      return;
    }

    mutation.mutate({ new_password: newPassword });
  };

  const handleStepUpSuccess = () => {
    setShowStepUp(false);
    mutation.mutate({ new_password: newPassword });
  };

  const handleRevokeOtherSessions = async () => {
    try {
      await accountClient.revokeOtherSessions();
      setShowSessionStepUp(false);
      setShowSessionPrompt(false);
      void queryClient.invalidateQueries({ queryKey: ['identity', 'sessions'] });
    } catch (err) {
      setError(clientErrorMessage(err, 'Impossible de déconnecter les autres sessions.'));
    }
  };

  return {
    accountEmail: me?.user.email ?? '',
    confirmPassword,
    error,
    handleSubmit,
    mutation,
    newPassword,
    setConfirmPassword,
    setNewPassword,
    showStepUp,
    setShowStepUp,
    handleStepUpSuccess,
    handleRevokeOtherSessions,
    requestResetEmail: () => {
      if (resetEmailResendSeconds === 0) {
        resetEmailMutation.mutate();
      }
    },
    resetEmailMutation,
    resetEmailResendSeconds,
    resetEmailSent,
    setShowSessionPrompt,
    setShowSessionStepUp,
    showSessionPrompt,
    showSessionStepUp,
    success,
    navigateBack: () =>
      void navigate({
        to: '/account/$accountIndex/security',
        params: { accountIndex: authuser },
      }),
  };
}
