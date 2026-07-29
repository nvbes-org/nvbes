import { clientErrorMessage } from '@nvbes/web-runtime';
import { useMutation } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import { clearCapturedAuthUrlToken, readCapturedAuthUrlToken } from '../identity.auth-url-secrets';
import {
  MAX_PASSWORD_LENGTH,
  MIN_PASSWORD_LENGTH,
  passwordHasSupportedLength,
} from '../identity.password.policy';
import { resetPassword } from '../identity.password.api';
import { estimatePasswordStrength } from './RegisterPage.password-strength';

export function useResetPasswordPage() {
  const navigate = useNavigate();
  const [tokenFromLink] = useState(() => readCapturedAuthUrlToken('/reset-password'));

  const [token, setToken] = useState(tokenFromLink);
  const [password, setPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const passwordResult = useMemo(() => {
    if (!password) {
      return null;
    }
    return estimatePasswordStrength(password);
  }, [password]);

  const mutation = useMutation({
    mutationFn: () => resetPassword(token, password),
    onSuccess: () => {
      clearCapturedAuthUrlToken('/reset-password');
      setSuccess(true);
    },
    onError: (error) =>
      setError(
        clientErrorMessage(error, 'Le lien est invalide ou a expire. Veuillez recommencer.'),
      ),
  });

  const handleSubmit = (event: React.SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);

    if (!passwordHasSupportedLength(password)) {
      setError(
        `Le mot de passe doit contenir entre ${MIN_PASSWORD_LENGTH} et ${MAX_PASSWORD_LENGTH} caractères.`,
      );
      return;
    }

    if ((passwordResult?.score ?? 0) < 3) {
      setError('Le mot de passe est trop faible. Veuillez choisir un mot de passe plus fort.');
      return;
    }

    if (password !== confirmPassword) {
      setError('Les mots de passe ne correspondent pas.');
      return;
    }

    mutation.mutate();
  };

  return {
    tokenFromLink,
    token,
    password,
    confirmPassword,
    error,
    success,
    isPending: mutation.isPending,
    navigateToLogin: () => void navigate({ to: '/login' }),
    setToken,
    setPassword,
    setConfirmPassword,
    handleSubmit,
  };
}
