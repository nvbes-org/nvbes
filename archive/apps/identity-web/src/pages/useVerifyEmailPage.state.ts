import { useLocation, useNavigate } from '@tanstack/react-router';
import { useEffect, useState } from 'react';
import {
  clearVerificationSnapshot,
  readVerificationSnapshot,
  saveVerificationSnapshot,
} from '../identity.email-verification.state';
import { readCapturedAuthUrlToken } from '../identity.auth-url-secrets';
import type { VerificationLocationState, VerificationStatus } from './VerifyEmailPage.shared';

export function useVerifyEmailPageState() {
  const navigate = useNavigate();
  const location = useLocation();
  const locationState = location.state as VerificationLocationState | null;
  const storedState = readVerificationSnapshot();

  const accountNameFromState = locationState?.accountName ?? storedState?.accountName ?? null;
  const emailFromState = locationState?.email ?? storedState?.email ?? '';
  const [token] = useState(
    () => readCapturedAuthUrlToken('/verify') || readCapturedAuthUrlToken('/verify-email') || null,
  );
  const resendAvailableAtFromState =
    locationState?.resendAvailableAt ?? storedState?.resendAvailableAt ?? null;

  const [status, setStatus] = useState<VerificationStatus>(token ? 'verifying' : 'pending');
  const [message, setMessage] = useState<string | null>(null);
  const [accountName] = useState(accountNameFromState);
  const [originalEmail, setOriginalEmail] = useState(emailFromState);
  const [emailDraft, setEmailDraft] = useState(emailFromState);
  const [resendAvailableAt, setResendAvailableAt] = useState<Date | null>(
    resendAvailableAtFromState ? new Date(resendAvailableAtFromState) : null,
  );
  const [now, setNow] = useState(() => new Date());
  const [working, setWorking] = useState(false);
  const [verified, setVerified] = useState(false);

  useEffect(() => {
    if (verified) {
      clearVerificationSnapshot();
      return;
    }

    saveVerificationSnapshot({
      accountName,
      email: originalEmail,
      resendAvailableAt: resendAvailableAt?.toISOString() ?? null,
    });
  }, [accountName, originalEmail, resendAvailableAt, verified]);

  return {
    accountName,
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
