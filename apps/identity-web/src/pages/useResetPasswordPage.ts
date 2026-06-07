import { useMutation } from '@tanstack/react-query';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import { resetPassword } from '../identity.password.api';

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

  const mutation = useMutation({
    mutationFn: () => resetPassword(token, password),
    onSuccess: () => setSuccess(true),
    onError: () => setError('Le lien est invalide ou a expire. Veuillez recommencer.'),
  });

  const handleSubmit = (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setError(null);

    if (password.length < 8) {
      setError('Le mot de passe doit contenir au moins 8 caracteres.');
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
