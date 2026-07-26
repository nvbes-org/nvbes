import { clientErrorMessage } from '@nvbes/web-runtime';
import { useMutation } from '@tanstack/react-query';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useMemo, useState } from 'react';
import { resetPassword } from '../identity.password.api';
import { estimatePasswordStrength } from './RegisterPage.password-strength';

export function useResetPasswordPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const searchParams = new URLSearchParams(location.searchStr);
  const tokenFromUrl = searchParams.get('token') ?? '';

  const [token, setToken] = useState(tokenFromUrl);
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
    onSuccess: () => setSuccess(true),
    onError: (error) =>
      setError(
        clientErrorMessage(error, 'Le lien est invalide ou a expire. Veuillez recommencer.'),
      ),
  });

  const handleSubmit = (event: React.SubmitEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);

    if (password.length < 8) {
      setError('Le mot de passe doit contenir au moins 8 caracteres.');
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
    tokenFromUrl,
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
