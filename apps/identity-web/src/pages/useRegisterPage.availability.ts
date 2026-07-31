import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import type { RegistrationAvailabilityField } from '../identity.auth.api';
import { identityAuthQueryKeys, registrationAvailabilityQueryFn } from '../identity.auth.queries';

const AVAILABILITY_DEBOUNCE_MS = 350;

export type RegistrationAvailabilityStatus =
  | 'idle'
  | 'checking'
  | 'available'
  | 'unavailable'
  | 'error';

export function useRegisterPageAvailability({
  email,
  emailValid,
  username,
  usernameValid,
}: {
  email: string;
  emailValid: boolean;
  username: string;
  usernameValid: boolean;
}) {
  const normalizedEmail = email.trim().toLowerCase();
  const normalizedUsername = username.trim().toLowerCase();
  const debouncedEmail = useDebouncedValue(normalizedEmail, AVAILABILITY_DEBOUNCE_MS);
  const debouncedUsername = useDebouncedValue(normalizedUsername, AVAILABILITY_DEBOUNCE_MS);

  const emailQuery = useAvailabilityQuery('email', debouncedEmail, emailValid);
  const usernameQuery = useAvailabilityQuery('username', debouncedUsername, usernameValid);

  return {
    emailAvailability: availabilityStatus(emailValid, normalizedEmail, debouncedEmail, emailQuery),
    usernameAvailability: availabilityStatus(
      usernameValid,
      normalizedUsername,
      debouncedUsername,
      usernameQuery,
    ),
  };
}

function useAvailabilityQuery(
  field: RegistrationAvailabilityField,
  value: string,
  locallyValid: boolean,
) {
  return useQuery({
    queryKey: identityAuthQueryKeys.registrationAvailability(field, value),
    queryFn: ({ signal }) => registrationAvailabilityQueryFn({ field, value, signal }),
    enabled: locallyValid && value !== '',
    retry: false,
    staleTime: 30_000,
  });
}

function availabilityStatus(
  locallyValid: boolean,
  currentValue: string,
  debouncedValue: string,
  query: ReturnType<typeof useAvailabilityQuery>,
): RegistrationAvailabilityStatus {
  if (!locallyValid) {
    return 'idle';
  }
  if (query.isError) {
    return 'error';
  }
  if (currentValue !== debouncedValue || query.isFetching || query.data === undefined) {
    return 'checking';
  }
  return query.data.available ? 'available' : 'unavailable';
}

function useDebouncedValue(value: string, delay: number): string {
  const [debouncedValue, setDebouncedValue] = useState(value);

  useEffect(() => {
    const timeoutId = globalThis.setTimeout(() => setDebouncedValue(value), delay);
    return () => globalThis.clearTimeout(timeoutId);
  }, [delay, value]);

  return debouncedValue;
}
