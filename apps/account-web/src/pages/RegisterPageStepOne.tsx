import { ArrowRightIcon } from 'lucide-react';
import { useEffect, useRef, type ReactNode, type SubmitEvent } from 'react';
import { BirthdateField } from '@/components/BirthdateField';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH } from '../identity.password.policy';

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

  useEffect(() => {
    if (emailError) {
      emailInputRef.current?.focus();
    }
  }, [emailError]);

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
        <BirthdateField
          id="register-birthdate"
          value={birthdate}
          min={minBirthdate}
          max={maxBirthdate}
          onChange={onBirthdateChange}
          required
        />
      </div>

      <div className="flex flex-col gap-2">
        <Label htmlFor="register-password">
          Mot de passe <span className="text-destructive">*</span>
        </Label>
        <Input
          id="register-password"
          name="password"
          type="password"
          placeholder="••••••••"
          value={password}
          onChange={(event) => onPasswordChange(event.target.value)}
          required
          autoComplete="new-password"
          minLength={MIN_PASSWORD_LENGTH}
          maxLength={MAX_PASSWORD_LENGTH}
          spellCheck={false}
        />
        <p className="text-xs text-muted-foreground">
          Utilisez au moins {MIN_PASSWORD_LENGTH} caractères. Les phrases de passe et gestionnaires
          de mots de passe sont acceptés.
        </p>
        {passwordStrength}
      </div>

      <Button type="submit" disabled={!canProceed} className="w-full" size="lg">
        Continuer vers la dernière étape
        <ArrowRightIcon data-icon="inline-end" />
      </Button>
    </form>
  );
}
