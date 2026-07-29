import { useMemo, useState } from 'react';
import { birthdateBounds } from '@/components/birthdate';
import { passwordHasSupportedLength } from '../identity.password.policy';
import { estimatePasswordStrength } from './RegisterPage.password-strength';
import type { RegisterStep } from './RegisterPage.shared';

export function useRegisterPageState() {
  const [step, setStep] = useState<RegisterStep>(1);
  const [firstname, setFirstname] = useState('');
  const [lastname, setLastname] = useState('');
  const [username, setUsername] = useState('');
  const [birthdate, setBirthdate] = useState('');
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

  return {
    birthdate,
    canProceedFromStep1:
      firstname.trim() !== '' &&
      lastname.trim() !== '' &&
      passwordHasSupportedLength(password) &&
      (passwordResult?.score ?? 0) >= 3,
    email,
    firstname,
    lastname,
    legalDocumentsAccepted,
    marketingEmailsAccepted,
    maxBirthdate: birthdateBounds().maxBirthdate,
    minBirthdate: birthdateBounds().minBirthdate,
    password,
    setBirthdate,
    setEmail,
    setFirstname,
    setLastname,
    setLegalDocumentsAccepted,
    setMarketingEmailsAccepted,
    setPassword,
    setStep,
    setUsername,
    step,
    username,
  };
}
