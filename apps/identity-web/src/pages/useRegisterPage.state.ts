import { useMemo, useState } from 'react';
import { emailHasSupportedFormat } from '../identity.email.policy';
import { passwordHasSupportedLength } from '../identity.password.policy';
import { estimatePasswordStrength } from '@/lib/password-strength';

export function useRegisterPageState() {
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [legalDocumentsAccepted, setLegalDocumentsAccepted] = useState(false);
  const [marketingEmailsAccepted, setMarketingEmailsAccepted] = useState(false);

  const passwordResult = useMemo(() => {
    if (!password) {
      return null;
    }

    return estimatePasswordStrength(password);
  }, [password]);

  const emailValid = emailHasSupportedFormat(email);
  const passwordValid = passwordHasSupportedLength(password) && (passwordResult?.score ?? 0) >= 3;
  return {
    canSubmit: emailValid && passwordValid && legalDocumentsAccepted,
    email,
    emailValid,
    legalDocumentsAccepted,
    marketingEmailsAccepted,
    password,
    passwordValid,
    setEmail,
    setLegalDocumentsAccepted,
    setMarketingEmailsAccepted,
    setPassword,
  };
}
