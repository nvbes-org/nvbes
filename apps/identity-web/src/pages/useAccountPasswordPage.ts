import { clientErrorMessage } from '@nvbes/web-runtime';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { type ChangePasswordInput, changePassword } from '../identity.password.api';

export function useAccountPasswordPage() {
  const queryClient = useQueryClient();
  const [currentPassword, setCurrentPassword] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const mutation = useMutation({
    mutationFn: (input: ChangePasswordInput) => changePassword(input),
    onSuccess: () => {
      setSuccess(true);
      setCurrentPassword('');
      setNewPassword('');
      setConfirmPassword('');
      void queryClient.invalidateQueries({ queryKey: ['identity', 'account'] });
    },
    onError: (error) => {
      setError(
        clientErrorMessage(
          error,
          'Le mot de passe actuel est incorrect ou le nouveau mot de passe est invalide.',
        ),
      );
    },
  });

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);
    setSuccess(false);

    if (newPassword.length < 8) {
      setError('Le nouveau mot de passe doit contenir au moins 8 caracteres.');
      return;
    }

    if (newPassword !== confirmPassword) {
      setError('Les mots de passe ne correspondent pas.');
      return;
    }

    if (currentPassword === newPassword) {
      setError("Le nouveau mot de passe doit etre different de l'actuel.");
      return;
    }

    mutation.mutate({ current_password: currentPassword, new_password: newPassword });
  };

  return {
    confirmPassword,
    currentPassword,
    error,
    handleSubmit,
    mutation,
    newPassword,
    setConfirmPassword,
    setCurrentPassword,
    setNewPassword,
    success,
  };
}
