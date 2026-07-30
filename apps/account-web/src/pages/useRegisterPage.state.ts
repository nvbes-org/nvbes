import { useMemo, useState } from 'react';
import { emailHasSupportedFormat } from '../identity.email.policy';
import { passwordHasSupportedLength } from '../identity.password.policy';
import { usernameHasSupportedLength } from '../identity.username.policy';
import { estimatePasswordStrength } from './RegisterPage.password-strength';

export function useRegisterPageState() {
  const [username, setUsername] = useState('');
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
  const usernameValid = usernameHasSupportedLength(username);

  return {
    canSubmit: emailValid && usernameValid && passwordValid && legalDocumentsAccepted,
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
    setUsername,
    username,
    usernameValid,
  };
}
