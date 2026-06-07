import { useLocation, useNavigate } from '@tanstack/react-router';
import { useState } from 'react';
import type { VerificationLocationState, VerificationStatus } from './VerifyEmailPage.shared';

export function useVerifyEmailPageState() {
  const navigate = useNavigate();
  const location = useLocation();
  const searchParams = new URLSearchParams(location.searchStr);
  const locationState = location.state as VerificationLocationState | null;

  const emailFromState = locationState?.email ?? '';
  const token = searchParams.get('token') ?? null;

  const [status, setStatus] = useState<VerificationStatus>(token ? 'verifying' : 'pending');
  const [message, setMessage] = useState<string | null>(null);
  const [originalEmail, setOriginalEmail] = useState(emailFromState);
  const [emailDraft, setEmailDraft] = useState(emailFromState);
  const [resendAvailableAt, setResendAvailableAt] = useState<Date | null>(
    locationState?.resendAvailableAt ? new Date(locationState.resendAvailableAt) : null,
  );
  const [now, setNow] = useState(() => new Date());
  const [working, setWorking] = useState(false);
  const [verified, setVerified] = useState(false);

  return {
    navigate,
    token,
    originalEmail,
    setOriginalEmail,
    emailDraft,
    setEmailDraft,
    resendAvailableAt,
    setResendAvailableAt,
    now,
    setNow,
    working,
    setWorking,
    verified,
    setVerified,
    status,
    setStatus,
    message,
    setMessage,
  };
}
