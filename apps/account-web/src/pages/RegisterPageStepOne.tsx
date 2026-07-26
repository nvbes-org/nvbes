import { ArrowRightIcon, CalendarIcon } from 'lucide-react';
import { useEffect, useMemo, useRef, useState, type ReactNode, type SubmitEvent } from 'react';
import type { Matcher } from 'react-day-picker';
import { Button } from '@/components/ui/button';
import { Calendar } from '@/components/ui/calendar';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';

function dateFromInputValue(value: string): Date | undefined {
  const match = /^(\d{4})-(\d{2})-(\d{2})$/u.exec(value);
  if (!match) {
    return undefined;
  }

  const [, year, month, day] = match;
  const date = new Date(Number(year), Number(month) - 1, Number(day));

  if (
    date.getFullYear() !== Number(year) ||
    date.getMonth() !== Number(month) - 1 ||
    date.getDate() !== Number(day)
  ) {
    return undefined;
  }

  return date;
}

function dateToInputValue(date: Date): string {
  const year = date.getFullYear().toString().padStart(4, '0');
  const month = (date.getMonth() + 1).toString().padStart(2, '0');
  const day = date.getDate().toString().padStart(2, '0');

  return `${year}-${month}-${day}`;
}

function displayValueFromInputValue(value: string): string {
  const date = dateFromInputValue(value);
  if (!date) {
    return '';
  }

  const day = date.getDate().toString().padStart(2, '0');
  const month = (date.getMonth() + 1).toString().padStart(2, '0');

  return `${day}/${month}/${date.getFullYear()}`;
}

function dateFromDisplayValue(value: string): Date | undefined {
  const match = /^(\d{2})\/(\d{2})\/(\d{4})$/u.exec(value);
  if (!match) {
    return undefined;
  }

  const [, day, month, year] = match;
  return dateFromInputValue(`${year}-${month}-${day}`);
}

function formatBirthdateDisplayValue(value: string): string {
  const digits = value.replace(/\D/gu, '').slice(0, 8);
  const day = digits.slice(0, 2);
  const month = digits.slice(2, 4);
  const year = digits.slice(4, 8);

  return [day, month, year].filter(Boolean).join('/');
}

function dateIsInRange(date: Date, min: Date | undefined, max: Date | undefined): boolean {
  return (!min || date >= min) && (!max || date <= max);
}

export function RegisterPageStepOne({
  firstname,
  lastname,
  username,
  birthdate,
  email,
  emailError,
  password,
  minBirthdate,
  maxBirthdate,
  canProceed,
  onFirstnameChange,
  onLastnameChange,
  onUsernameChange,
  onBirthdateChange,
  onEmailChange,
  onPasswordChange,
  onSubmit,
  passwordStrength,
}: {
  firstname: string;
  lastname: string;
  username: string;
  birthdate: string;
  email: string;
  emailError: boolean;
  password: string;
  minBirthdate: string;
  maxBirthdate: string;
  canProceed: boolean;
  onFirstnameChange: (value: string) => void;
  onLastnameChange: (value: string) => void;
  onUsernameChange: (value: string) => void;
  onBirthdateChange: (value: string) => void;
  onEmailChange: (value: string) => void;
  onPasswordChange: (value: string) => void;
  onSubmit: (event: SubmitEvent<HTMLFormElement>) => void;
  passwordStrength: ReactNode;
}) {
  const emailInputRef = useRef<HTMLInputElement>(null);
  const [birthdatePickerOpen, setBirthdatePickerOpen] = useState(false);
  const [birthdateDisplay, setBirthdateDisplay] = useState(() =>
    displayValueFromInputValue(birthdate),
  );
  const selectedBirthdate = useMemo(() => dateFromInputValue(birthdate), [birthdate]);
  const minBirthdateDate = useMemo(() => dateFromInputValue(minBirthdate), [minBirthdate]);
  const maxBirthdateDate = useMemo(() => dateFromInputValue(maxBirthdate), [maxBirthdate]);
  const disabledBirthdateMatchers = useMemo<Matcher[]>(() => {
    const matchers: Matcher[] = [];

    if (minBirthdateDate) {
      matchers.push({ before: minBirthdateDate });
    }

    if (maxBirthdateDate) {
      matchers.push({ after: maxBirthdateDate });
    }

    return matchers;
  }, [minBirthdateDate, maxBirthdateDate]);

  useEffect(() => {
    setBirthdateDisplay(displayValueFromInputValue(birthdate));
  }, [birthdate]);

  useEffect(() => {
    if (emailError) {
      emailInputRef.current?.focus();
    }
  }, [emailError]);

  const updateBirthdateFromDisplayValue = (value: string, input: HTMLInputElement | null) => {
    const formattedValue = formatBirthdateDisplayValue(value);
    setBirthdateDisplay(formattedValue);
    input?.setCustomValidity('');

    if (!formattedValue) {
      onBirthdateChange('');
      return;
    }

    const date = dateFromDisplayValue(formattedValue);
    if (!date) {
      if (formattedValue.length >= 10) {
        onBirthdateChange('');
        input?.setCustomValidity('Date de naissance invalide.');
      }
      return;
    }

    if (!dateIsInRange(date, minBirthdateDate, maxBirthdateDate)) {
      onBirthdateChange('');
      input?.setCustomValidity('Date de naissance hors limites.');
      return;
    }

    onBirthdateChange(dateToInputValue(date));
  };

  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-5">
      <div className="grid grid-cols-2 gap-3">
        <div className="flex flex-col gap-2">
          <Label htmlFor="register-firstname">
            Prénom <span className="text-destructive">*</span>
          </Label>
          <Input
            id="register-firstname"
            type="text"
            placeholder="Prénom"
            value={firstname}
            onChange={(event) => onFirstnameChange(event.target.value)}
            required
          />
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="register-lastname">
            Nom <span className="text-destructive">*</span>
          </Label>
          <Input
            id="register-lastname"
            type="text"
            placeholder="Nom"
            value={lastname}
            onChange={(event) => onLastnameChange(event.target.value)}
            required
          />
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <Label htmlFor="register-email">
          Email <span className="text-destructive">*</span>
        </Label>
        <Input
          id="register-email"
          ref={emailInputRef}
          type="email"
          placeholder="vous@exemple.fr"
          value={email}
          onChange={(event) => onEmailChange(event.target.value)}
          required
          autoComplete="email"
          autoFocus
          aria-describedby={emailError ? 'register-email-error' : undefined}
          aria-invalid={emailError}
          className={emailError ? 'border-destructive focus-visible:ring-destructive' : undefined}
        />
        {emailError && (
          <p id="register-email-error" className="text-sm text-destructive" role="alert">
            Cet email est déjà utilisé. Connectez-vous ou choisissez une autre adresse.
          </p>
        )}
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div className="flex flex-col gap-2">
          <Label htmlFor="register-username">
            Nom d&apos;utilisateur <span className="text-destructive">*</span>
          </Label>
          <Input
            id="register-username"
            type="text"
            placeholder="Nom d'utilisateur"
            value={username}
            onChange={(event) => onUsernameChange(event.target.value)}
            required
            autoComplete="username"
          />
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="register-birthdate">
            Date de naissance <span className="text-destructive">*</span>
          </Label>
          <div className="relative">
            <Input
              id="register-birthdate"
              type="text"
              placeholder="dd/mm/yyyy"
              value={birthdateDisplay}
              onChange={(event) =>
                updateBirthdateFromDisplayValue(event.target.value, event.currentTarget)
              }
              onBlur={(event) =>
                updateBirthdateFromDisplayValue(event.target.value, event.currentTarget)
              }
              inputMode="numeric"
              pattern="\d{2}/\d{2}/\d{4}"
              required
              autoComplete="bday"
              className="pr-10"
            />
            <Popover open={birthdatePickerOpen} onOpenChange={setBirthdatePickerOpen}>
              <PopoverTrigger asChild>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon"
                  className="absolute top-1/2 right-1 -translate-y-1/2"
                  aria-label="Ouvrir le calendrier"
                >
                  <CalendarIcon />
                </Button>
              </PopoverTrigger>
              <PopoverContent className="w-auto p-0" align="end">
                <Calendar
                  mode="single"
                  selected={selectedBirthdate}
                  onSelect={(date) => {
                    if (!date) {
                      return;
                    }

                    onBirthdateChange(dateToInputValue(date));
                    setBirthdatePickerOpen(false);
                  }}
                  captionLayout="dropdown"
                  startMonth={minBirthdateDate}
                  endMonth={maxBirthdateDate}
                  disabled={disabledBirthdateMatchers}
                />
              </PopoverContent>
            </Popover>
          </div>
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <Label htmlFor="register-password">
          Mot de passe <span className="text-destructive">*</span>
        </Label>
        <Input
          id="register-password"
          type="password"
          placeholder="••••••••"
          value={password}
          onChange={(event) => onPasswordChange(event.target.value)}
          required
          autoComplete="new-password"
        />
        {passwordStrength}
      </div>

      <Button type="submit" disabled={!canProceed} className="w-full" size="lg">
        Continuer vers la dernière étape
        <ArrowRightIcon data-icon="inline-end" />
      </Button>
    </form>
  );
}
