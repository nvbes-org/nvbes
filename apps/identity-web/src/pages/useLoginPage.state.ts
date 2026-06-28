import { useState } from 'react';

import type { MfaMethod } from './LoginPage.mfa';
import type { LoginStep } from './LoginProgress';

export function useLoginPageState() {
  const [checkingAuth, setCheckingAuth] = useState(true);
  const [connectedAccounts, setConnectedAccounts] = useState<
    import('@nvbes/identity-client').AccountEntry[]
  >([]);
  const [step, setStep] = useState<LoginStep>('identifier');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [identifierSubmitting, setIdentifierSubmitting] = useState(false);
  const [loginStateToken, setLoginStateToken] = useState<string | null>(null);
  const [sessionToken, setSessionToken] = useState<string | null>(null);
  const [availableMethods, setAvailableMethods] = useState<MfaMethod[]>([]);
  const [mfaMethod, setMfaMethod] = useState<MfaMethod | null>(null);
  const [totpCode, setTotpCode] = useState('');
  const [emailCode, setEmailCode] = useState('');
  const [recoveryCode, setRecoveryCode] = useState('');
  const [error, setError] = useState<string | null>(null);

  const resetMfaState = () => {
    setAvailableMethods([]);
    setMfaMethod(null);
    setTotpCode('');
    setEmailCode('');
    setRecoveryCode('');
  };

  return {
    checkingAuth,
    connectedAccounts,
    step,
    email,
    password,
    identifierSubmitting,
    loginStateToken,
    sessionToken,
    availableMethods,
    mfaMethod,
    totpCode,
    emailCode,
    recoveryCode,
    error,
    setCheckingAuth,
    setConnectedAccounts,
    setStep,
    setEmail,
    setPassword,
    setIdentifierSubmitting,
    setLoginStateToken,
    setSessionToken,
    setAvailableMethods,
    setMfaMethod,
    setTotpCode,
    setEmailCode,
    setRecoveryCode,
    setError,
    resetMfaState,
    hasRecovery: availableMethods.includes('recovery'),
    hasTotp: availableMethods.includes('totp'),
    hasEmail: availableMethods.includes('email'),
    hasWebAuthn: availableMethods.includes('webauthn'),
    availableCount: availableMethods.length,
  };
}
