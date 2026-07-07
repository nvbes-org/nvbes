import { createRequestHeaders } from '@nvbes/http-client';
import { verifiedFetch } from '@nvbes/web-runtime';
import type { DeviceActivationStep, DeviceInfo } from './DeviceActivationPage.shared';
import { getDeviceActionEndpoint } from './useDeviceActivationPage.shared';

export async function verifyDeviceUserCode({
  code,
  setDeviceInfo,
  setStep,
  setError,
}: {
  code: string;
  setDeviceInfo: React.Dispatch<React.SetStateAction<DeviceInfo | null>>;
  setStep: React.Dispatch<React.SetStateAction<DeviceActivationStep>>;
  setError: (value: string) => void;
}) {
  const response = await verifiedFetch('/oauth/device/verify', {
    method: 'POST',
    headers: createRequestHeaders('POST', {
      'Content-Type': 'application/json',
    }),
    body: JSON.stringify({ user_code: code }),
  });

  if (response.ok) {
    const data = await response.json();
    setDeviceInfo(data);
    setStep('approve');
    return;
  }

  const data = await response.json();
  setError(data.message || 'Code invalide ou expiré');
}

export async function handleDeviceConsentAction({
  action,
  csrfToken,
  selectedWorkspace,
  setError,
  setStep,
  userCode,
}: {
  action: 'approve' | 'deny';
  csrfToken: string | undefined;
  selectedWorkspace: string;
  setError: (value: string) => void;
  setStep: React.Dispatch<React.SetStateAction<DeviceActivationStep>>;
  userCode: string;
}) {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };
  if (csrfToken) {
    headers['X-CSRF-Token'] = csrfToken;
  }

  const response = await verifiedFetch(getDeviceActionEndpoint(action), {
    method: 'POST',
    headers: createRequestHeaders('POST', headers),
    body: JSON.stringify({
      user_code: userCode,
      workspace_id: selectedWorkspace,
      consent_action: action,
    }),
    credentials: 'include',
  });

  if (response.ok) {
    setStep('success');
    return;
  }

  const data = await response.json();
  setError(data.message || 'Une erreur est survenue');
}
