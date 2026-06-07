import { type AccountWorkspace, identityClient } from '@nvbes/identity-client';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useCallback, useEffect, useState, type FormEvent } from 'react';

import { useAccountContext } from '@/hooks/useAccountContext';
import type { DeviceActivationStep, DeviceInfo } from './DeviceActivationPage.shared';
import { handleDeviceConsentAction, verifyDeviceUserCode } from './useDeviceActivationPage.actions';
import { loadDeviceActivationWorkspaces } from './useDeviceActivationPage.bootstrap';
import { csrfTokenFromCookie } from './useDeviceActivationPage.shared';

export function useDeviceActivationPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const searchParams = new URLSearchParams(location.searchStr);
  const { me } = useAccountContext();
  const initialUserCode = searchParams.get('user_code') || '';

  const [step, setStep] = useState<DeviceActivationStep>('input');
  const [userCode, setUserCode] = useState(initialUserCode);
  const [deviceInfo, setDeviceInfo] = useState<DeviceInfo | null>(null);
  const [workspaces, setWorkspaces] = useState<AccountWorkspace[]>([]);
  const [selectedWorkspace, setSelectedWorkspace] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  const verifyUserCode = useCallback(async (code: string) => {
    setLoading(true);
    setError('');

    try {
      await verifyDeviceUserCode({
        code,
        setDeviceInfo,
        setStep,
        setError,
      });
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
        await loadDeviceActivationWorkspaces({
          currentWorkspaceId: me?.current_workspace_id,
          listWorkspaces: () => identityClient.listWorkspaces(),
          setSelectedWorkspace,
          setWorkspaces,
        });
      } catch (err) {
        console.error('Failed to fetch workspaces', err);
      }
    };

    void fetchWorkspaces();

    if (initialUserCode) {
      void verifyUserCode(initialUserCode);
    }
  }, [initialUserCode, me?.current_workspace_id, verifyUserCode]);

  const handleVerify = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    void verifyUserCode(userCode);
  };

  const handleAction = async (action: 'approve' | 'deny') => {
    setLoading(true);
    setError('');

    try {
      await handleDeviceConsentAction({
        action,
        csrfToken: csrfTokenFromCookie(),
        selectedWorkspace,
        setError,
        setStep,
        userCode,
      });
    } catch (err) {
      console.error('Failed to handle device action:', err);
      setError('Erreur de connexion au serveur');
    } finally {
      setLoading(false);
    }
  };

  return {
    deviceInfo,
    error,
    handleApprove: () => void handleAction('approve'),
    handleDeny: () => void handleAction('deny'),
    handleVerify,
    loading,
    lockedWorkspace: Boolean(me?.current_workspace_id),
    navigate,
    selectedWorkspace,
    setSelectedWorkspace,
    setUserCode,
    step,
    userCode,
    workspaces,
  };
}
