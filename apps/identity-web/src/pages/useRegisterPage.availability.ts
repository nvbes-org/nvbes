import { useQuery } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
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
}: {
  email: string;
  emailValid: boolean;
}) {
  const normalizedEmail = email.trim().toLowerCase();
  const debouncedEmail = useDebouncedValue(normalizedEmail, AVAILABILITY_DEBOUNCE_MS);

  const emailQuery = useAvailabilityQuery(debouncedEmail, emailValid);

  return {
    emailAvailability: availabilityStatus(emailValid, normalizedEmail, debouncedEmail, emailQuery),
  };
}

function useAvailabilityQuery(email: string, locallyValid: boolean) {
  return useQuery({
    queryKey: identityAuthQueryKeys.registrationAvailability(email),
    queryFn: ({ signal }) => registrationAvailabilityQueryFn({ email, signal }),
    enabled: locallyValid && email !== '',
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
