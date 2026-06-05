import { type AccountWorkspace, identityClient } from '@nvbes/identity-client';
import { useLocation, useNavigate } from '@tanstack/react-router';
import type { ChangeEvent } from 'react';
import { useCallback, useEffect, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useAccountContext } from '@/hooks/useAccountContext';

interface DeviceInfo {
  client_name: string;
  scope: string[];
  tenant_id: string;
}

export default function DeviceActivationPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const searchParams = new URLSearchParams(location.searchStr);
  const { me } = useAccountContext();
  const [step, setStep] = useState<'input' | 'approve' | 'success' | 'error'>('input');
  const initialUserCode = searchParams.get('user_code') || '';
  const [userCode, setUserCode] = useState(initialUserCode);
  const [deviceInfo, setDeviceInfo] = useState<DeviceInfo | null>(null);
  const [workspaces, setWorkspaces] = useState<AccountWorkspace[]>([]);
  const [selectedWorkspace, setSelectedWorkspace] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  // Pre-fetch workspaces if logged in
  const verifyUserCode = useCallback(async (code: string) => {
    setLoading(true);
    setError('');

    try {
      const response = await fetch('/oauth/device/verify', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ user_code: code }),
      });

      if (response.ok) {
        const data = await response.json();
        setDeviceInfo(data);
        setStep('approve');
      } else {
        const data = await response.json();
        setError(data.message || 'Code invalide ou expiré');
      }
    } catch (err) {
      console.error('Failed to verify user code:', err);
      setError('Erreur de connexion au serveur');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    const fetchWorkspaces = async () => {
      try {
        const wsList = await identityClient.listWorkspaces();
        setWorkspaces(wsList);
        if (me?.current_workspace_id) {
          setSelectedWorkspace(me.current_workspace_id);
        } else if (wsList.length > 0) {
          setSelectedWorkspace(wsList[0].id);
        }
      } catch (err) {
        console.error('Failed to fetch workspaces', err);
      }
    };
    void fetchWorkspaces();

    if (initialUserCode) {
      void verifyUserCode(initialUserCode);
    }
  }, [initialUserCode, me?.current_workspace_id, verifyUserCode]);

  const handleVerify = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    void verifyUserCode(userCode);
  };

  const handleAction = async (action: 'approve' | 'deny') => {
    setLoading(true);
    setError('');
    const endpoint = action === 'approve' ? '/oauth/device/approve' : '/oauth/device/deny';

    const csrfMatch =
      typeof document !== 'undefined'
        ? document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/)
        : null;
    const csrfToken = csrfMatch?.[1];
    const headers: Record<string, string> = { 'Content-Type': 'application/json' };
    if (csrfToken) {
      headers['X-CSRF-Token'] = csrfToken;
    }

    try {
      const response = await fetch(endpoint, {
        method: 'POST',
        headers,
        body: JSON.stringify({
          user_code: userCode,
          workspace_id: selectedWorkspace,
          consent_action: action === 'approve' ? 'approve' : 'deny',
        }),
        credentials: 'include',
      });

      if (response.ok) {
        setStep('success');
      } else {
        const data = await response.json();
        setError(data.message || 'Une erreur est survenue');
      }
    } catch (err) {
      console.error('Failed to handle device action:', err);
      setError('Erreur de connexion au serveur');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex min-h-screen flex-col items-center justify-center bg-background p-4">
      <div className="w-full max-w-md space-y-8 rounded-2xl border bg-card p-8 shadow-xl">
        <div className="text-center">
          <h1 className="text-3xl font-extrabold tracking-tight">Activation de l'appareil</h1>
          <p className="mt-2 text-sm text-muted-foreground">
            Connectez votre appareil à votre compte nvbes.
          </p>
        </div>

        {error && (
          <div className="rounded-lg bg-destructive/10 p-3 text-sm text-destructive">{error}</div>
        )}

        {step === 'input' && (
          <form onSubmit={handleVerify} className="space-y-6">
            <div className="space-y-2">
              <label htmlFor="userCode" className="text-sm font-medium">
                Code d'activation
              </label>
              <Input
                id="userCode"
                placeholder="XXXX-XXXX"
                value={userCode}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  setUserCode(e.target.value.toUpperCase())
                }
                className="text-center text-2xl font-mono tracking-widest"
                required
              />
            </div>
            <Button type="submit" disabled={loading} className="w-full h-12 text-lg">
              {loading ? 'Vérification...' : 'Continuer'}
            </Button>
          </form>
        )}

        {step === 'approve' && deviceInfo && (
          <div className="space-y-6">
            <div className="rounded-lg bg-muted p-4 space-y-2">
              <p className="text-sm font-medium">
                Application: <span className="font-bold">{deviceInfo.client_name}</span>
              </p>
              <p className="text-xs text-muted-foreground">
                Demande l'accès aux permissions suivantes:
              </p>
              <ul className="list-disc list-inside text-xs space-y-1">
                {deviceInfo.scope.map((s) => (
                  <li key={s}>{s}</li>
                ))}
              </ul>
            </div>

            <div className="space-y-2">
              <label htmlFor="workspace-select" className="text-sm font-medium">
                Workspace courant
              </label>
              <select
                id="workspace-select"
                value={selectedWorkspace}
                onChange={(e: React.ChangeEvent<HTMLSelectElement>) =>
                  setSelectedWorkspace(e.target.value)
                }
                disabled={Boolean(me?.current_workspace_id)}
                className="w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
              >
                {workspaces.map((ws) => (
                  <option key={ws.id} value={ws.id}>
                    {ws.name}
                  </option>
                ))}
              </select>
              {me?.current_workspace_id ? (
                <p className="text-xs text-muted-foreground">
                  Le workspace courant est utilisé automatiquement.
                </p>
              ) : null}
            </div>

            <div className="grid grid-cols-2 gap-4">
              <Button
                variant="outline"
                onClick={() => handleAction('deny')}
                disabled={loading}
                className="h-12"
              >
                Refuser
              </Button>
              <Button
                onClick={() => handleAction('approve')}
                disabled={loading || !selectedWorkspace}
                className="h-12"
              >
                Approuver
              </Button>
            </div>
          </div>
        )}

        {step === 'success' && (
          <div className="text-center space-y-6 py-4">
            <div className="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-green-100">
              <svg
                className="h-8 w-8 text-green-600"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                role="img"
                aria-label="Succès"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M5 13l4 4L19 7"
                />
              </svg>
            </div>
            <div className="space-y-2">
              <h2 className="text-xl font-bold">Appareil activé !</h2>
              <p className="text-sm text-muted-foreground">
                Vous pouvez maintenant retourner sur votre appareil pour continuer.
              </p>
            </div>
            <Button
              variant="outline"
              onClick={() => void navigate({ to: '/account' })}
              className="w-full"
            >
              Aller à mon compte
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
