import { zxcvbn } from '@zxcvbn-ts/core';
import { useMemo, useState } from 'react';
import type { RegisterStep } from './RegisterPage.shared';

function birthdateBounds() {
  const today = new Date();
  return {
    minBirthdate: new Date(today.getFullYear() - 120, today.getMonth(), today.getDate())
      .toISOString()
      .split('T')[0],
    maxBirthdate: new Date(today.getFullYear() - 13, today.getMonth(), today.getDate())
      .toISOString()
      .split('T')[0],
  };
}

export function useRegisterPageState() {
  const [step, setStep] = useState<RegisterStep>(1);
  const [firstname, setFirstname] = useState('');
  const [lastname, setLastname] = useState('');
  const [username, setUsername] = useState('');
  const [birthdate, setBirthdate] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [workspaceName, setWorkspaceName] = useState('');

  const passwordResult = useMemo(() => {
    if (!password) {
      return null;
    }

    return zxcvbn(password);
  }, [password]);

  return {
    birthdate,
    canProceedFromStep1: (passwordResult?.score ?? 0) >= 3,
    email,
    firstname,
    lastname,
    maxBirthdate: birthdateBounds().maxBirthdate,
    minBirthdate: birthdateBounds().minBirthdate,
    password,
    setBirthdate,
    setEmail,
    setFirstname,
    setLastname,
    setPassword,
    setStep,
    setUsername,
    setWorkspaceName,
    step,
    username,
    workspaceName,
  };
}
