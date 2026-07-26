import { clientErrorMessage } from '@nvbes/web-runtime';
import { storePasswordCredential } from '@nvbes/identity-sdk-web';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import { useAccountContext } from '@/hooks/useAccountContext';
import { readAuthuser } from '@/identity.authuser';
import { type ChangePasswordInput, changePassword } from '../identity.password.api';
import { estimatePasswordStrength } from './RegisterPage.password-strength';

import { isStepUpRequiredError } from '@/identity.step-up';

export function useAccountPasswordPage() {
  const location = useLocation();
  const authuser = readAuthuser(location.searchStr, location.pathname);
  const { me } = useAccountContext();
  const navigate = useNavigate();
  const queryClient = useQueryClient();
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const [showStepUp, setShowStepUp] = useState(false);

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

  const handleSubmit = async (event: React.SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setSuccess(false);

    if (newPassword.length < 8) {
      setError('Le nouveau mot de passe doit contenir au moins 8 caracteres.');
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
    success,
    navigateBack: () =>
      void navigate({
        to: '/account/$accountIndex/security',
        params: { accountIndex: authuser },
      }),
  };
}
