import { CheckIcon } from 'lucide-react';
import { useEffect, useRef, useState, type ReactNode } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
import { Label } from '@/components/ui/label';
import { Spinner } from '@/components/ui/spinner';
import { MAX_PASSWORD_LENGTH, MIN_PASSWORD_LENGTH } from '../identity.password.policy';
import { MAX_USERNAME_LENGTH } from '../identity.username.policy';
import { registrationAvailabilityMessage, ValidatedInput } from './RegisterPage.field';
import { PasswordStrengthMeter } from './RegisterPage.shared';
import type { useRegisterPage } from './useRegisterPage';

type RegisterFormProps = ReturnType<typeof useRegisterPage>;

export function RegisterForm({
  canSubmit,
  email,
  emailAlreadyExists,
  emailAvailability,
  emailValid,
  error,
  handleEmailChange,
  handleSubmit,
  handleUsernameChange,
  legalDocumentsAccepted,
  loading,
  marketingEmailsAccepted,
  password,
  passwordValid,
  setLegalDocumentsAccepted,
  setMarketingEmailsAccepted,
  setPassword,
  username,
  usernameTaken,
  usernameAvailability,
  usernameValid,
}: RegisterFormProps) {
  const emailInputRef = useRef<HTMLInputElement>(null);
  const usernameInputRef = useRef<HTMLInputElement>(null);
  const [emailTouched, setEmailTouched] = useState(false);
  const [passwordTouched, setPasswordTouched] = useState(false);
  const [usernameTouched, setUsernameTouched] = useState(false);

  const emailInvalid =
    emailAlreadyExists || (emailTouched && (!emailValid || emailAvailability === 'unavailable'));
  const emailCorrect =
    emailTouched && emailValid && !emailAlreadyExists && emailAvailability === 'available';
  const emailChecking = emailTouched && emailValid && emailAvailability === 'checking';
  const usernameInvalid =
    usernameTaken ||
    (usernameTouched && (!usernameValid || usernameAvailability === 'unavailable'));
  const usernameCorrect =
    usernameTouched && usernameValid && !usernameTaken && usernameAvailability === 'available';
  const usernameChecking = usernameTouched && usernameValid && usernameAvailability === 'checking';
  const passwordInvalid = passwordTouched && !passwordValid;
  const passwordCorrect = passwordTouched && passwordValid;

  useEffect(() => {
    if (emailAlreadyExists) {
      emailInputRef.current?.focus();
    } else if (usernameTaken) {
      usernameInputRef.current?.focus();
    }
  }, [emailAlreadyExists, usernameTaken]);

  return (
    <form onSubmit={handleSubmit} autoComplete="on" className="flex flex-col gap-5">
      <div className="flex flex-col gap-2">
        <Label htmlFor="register-email">
          Email <span className="text-destructive">*</span>
        </Label>
        <ValidatedInput
          id="register-email"
          name="email"
          inputRef={emailInputRef}
          type="email"
          placeholder="vous@exemple.fr"
          value={email}
          onChange={(event) => handleEmailChange(event.target.value)}
          onBlur={() => setEmailTouched(true)}
          required
          autoComplete="username"
          autoFocus
          correct={emailCorrect}
          invalid={emailInvalid}
          checking={emailChecking}
          message={
            emailAlreadyExists
              ? 'Cet email est déjà utilisé. Connectez-vous ou choisissez une autre adresse.'
              : registrationAvailabilityMessage({
                  availability: emailAvailability,
                  availableMessage: 'Adresse email valide.',
                  unavailableMessage: 'Cet email est déjà utilisé.',
                  checkingMessage: "Vérification de l'email...",
                  invalidMessage: 'Saisissez une adresse email valide.',
                  invalid: emailInvalid,
                  touched: emailTouched,
                })
          }
        />
      </div>

      <div className="flex flex-col gap-2">
        <Label htmlFor="register-username">
          Nom d&apos;utilisateur <span className="text-destructive">*</span>
        </Label>
        <ValidatedInput
          id="register-username"
          name="nickname"
          inputRef={usernameInputRef}
          type="text"
          placeholder="username"
          value={username}
          onChange={(event) => handleUsernameChange(event.target.value)}
          onBlur={() => setUsernameTouched(true)}
          required
          autoComplete="nickname"
          maxLength={MAX_USERNAME_LENGTH}
          spellCheck={false}
          correct={usernameCorrect}
          invalid={usernameInvalid}
          checking={usernameChecking}
          message={
            usernameTaken
              ? "Ce nom d'utilisateur est déjà pris."
              : registrationAvailabilityMessage({
                  availability: usernameAvailability,
                  availableMessage: "Nom d'utilisateur valide.",
                  unavailableMessage: "Ce nom d'utilisateur est déjà pris.",
                  checkingMessage: "Vérification du nom d'utilisateur...",
                  invalidMessage: "Saisissez un nom d'utilisateur.",
                  invalid: usernameInvalid,
                  touched: usernameTouched,
                })
          }
        />
      </div>

      <div className="flex flex-col gap-2">
        <Label htmlFor="register-password">
          Mot de passe <span className="text-destructive">*</span>
        </Label>
        <ValidatedInput
          id="register-password"
          name="password"
          type="password"
          placeholder="••••••••"
          value={password}
          onChange={(event) => setPassword(event.target.value)}
          onBlur={() => setPasswordTouched(true)}
          required
          autoComplete="new-password"
          minLength={MIN_PASSWORD_LENGTH}
          maxLength={MAX_PASSWORD_LENGTH}
          spellCheck={false}
          correct={passwordCorrect}
          invalid={passwordInvalid}
          message={
            passwordCorrect
              ? 'Mot de passe valide.'
              : passwordInvalid
                ? password.length < MIN_PASSWORD_LENGTH
                  ? `Utilisez au moins ${MIN_PASSWORD_LENGTH} caractères.`
                  : 'Choisissez un mot de passe plus robuste.'
                : `Utilisez au moins ${MIN_PASSWORD_LENGTH} caractères.`
          }
        />
        <PasswordStrengthMeter password={password} />
      </div>

      <div className="flex flex-col gap-3 border-t border-border pt-4">
        <ConsentCheckbox
          id="register-legal-documents"
          checked={legalDocumentsAccepted}
          onCheckedChange={setLegalDocumentsAccepted}
        >
          J&apos;accepte les <LegalLink href="/legal/terms-of-service">Conditions</LegalLink>, la{' '}
          <LegalLink href="/legal/privacy-policy">Confidentialité</LegalLink> et le{' '}
          <LegalLink href="/legal/data-processing-agreement">DPA</LegalLink>.{' '}
          <span className="text-destructive">*</span>
        </ConsentCheckbox>
        <ConsentCheckbox
          id="register-marketing-emails"
          checked={marketingEmailsAccepted}
          onCheckedChange={setMarketingEmailsAccepted}
          muted
        >
          Je souhaite recevoir les nouveautés et conseils nvbes par email.
        </ConsentCheckbox>
      </div>

      {error && !emailAlreadyExists && !usernameTaken && (
        <Alert variant="destructive">
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      )}

      <Button type="submit" className="w-full" size="lg" disabled={loading || !canSubmit}>
        {loading ? (
          <>
            <Spinner data-icon="inline-start" />
            Inscription...
          </>
        ) : (
          <>
            Créer mon compte
            <CheckIcon data-icon="inline-end" />
          </>
        )}
      </Button>
    </form>
  );
}

function ConsentCheckbox({
  id,
  checked,
  onCheckedChange,
  muted = false,
  children,
}: {
  id: string;
  checked: boolean;
  onCheckedChange: (value: boolean) => void;
  muted?: boolean;
  children: ReactNode;
}) {
  return (
    <div
      className={`flex items-start gap-3 text-sm leading-5${muted ? ' text-muted-foreground' : ''}`}
    >
      <Checkbox
        id={id}
        checked={checked}
        onCheckedChange={(value) => onCheckedChange(value === true)}
        className="mt-0.5"
      />
      <label htmlFor={id}>{children}</label>
    </div>
  );
}

function LegalLink({ href, children }: { href: string; children: ReactNode }) {
  return (
    <a href={href} className="font-medium text-primary hover:underline">
      {children}
    </a>
  );
}
