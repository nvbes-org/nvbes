import { CircleCheckIcon, CircleXIcon } from 'lucide-react';
import type { ComponentProps, Ref } from 'react';
import { Input } from '@/components/ui/input';
import { Spinner } from '@/components/ui/spinner';
import type { RegistrationAvailabilityStatus } from './useRegisterPage.availability';

export function ValidatedInput({
  id,
  inputRef,
  correct,
  invalid,
  checking = false,
  message,
  className,
  ...props
}: ComponentProps<typeof Input> & {
  id: string;
  inputRef?: Ref<HTMLInputElement>;
  correct: boolean;
  invalid: boolean;
  checking?: boolean;
  message?: string;
}) {
  const messageId = `${id}-validation`;
  const StatusIcon = correct ? CircleCheckIcon : CircleXIcon;

  return (
    <>
      <div className="relative">
        <Input
          ref={inputRef}
          id={id}
          className={`${className ?? ''} pr-9 ${
            correct
              ? 'border-emerald-600 focus-visible:border-emerald-600 focus-visible:ring-emerald-600/20'
              : ''
          }`}
          aria-describedby={message ? messageId : undefined}
          aria-invalid={invalid}
          {...props}
        />
        {checking && (
          <Spinner
            className="pointer-events-none absolute top-1/2 right-2.5 size-4 -translate-y-1/2"
            aria-hidden="true"
          />
        )}
        {!checking && (correct || invalid) && (
          <StatusIcon
            className={`pointer-events-none absolute top-1/2 right-2.5 size-4 -translate-y-1/2 ${
              correct ? 'text-emerald-600' : 'text-destructive'
            }`}
            aria-hidden="true"
          />
        )}
      </div>
      {message && (
        <p
          id={messageId}
          className={`text-xs ${
            correct ? 'text-emerald-700' : invalid ? 'text-destructive' : 'text-muted-foreground'
          }`}
          role={invalid ? 'alert' : 'status'}
        >
          {message}
        </p>
      )}
    </>
  );
}

export function registrationAvailabilityMessage({
  availability,
  availableMessage,
  unavailableMessage,
  checkingMessage,
  invalidMessage,
  invalid,
  touched,
}: {
  availability: RegistrationAvailabilityStatus;
  availableMessage: string;
  unavailableMessage: string;
  checkingMessage: string;
  invalidMessage: string;
  invalid: boolean;
  touched: boolean;
}): string | undefined {
  if (!touched) {
    return undefined;
  }
  if (availability === 'available') {
    return availableMessage;
  }
  if (availability === 'unavailable') {
    return unavailableMessage;
  }
  if (availability === 'checking') {
    return checkingMessage;
  }
  if (availability === 'error') {
    return 'Vérification indisponible. Vous pouvez continuer.';
  }
  return invalid ? invalidMessage : undefined;
}
