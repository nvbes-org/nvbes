import { ArrowRightIcon } from 'lucide-react';
import type { FormEvent, ReactNode } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';

export function RegisterPageStepOne({
  firstname,
  lastname,
  username,
  birthdate,
  email,
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
  onSubmit: (event: FormEvent<HTMLFormElement>) => void;
  passwordStrength: ReactNode;
}) {
  return (
    <form onSubmit={onSubmit} className="flex flex-col gap-5">
      <div className="grid grid-cols-2 gap-3">
        <div className="flex flex-col gap-2">
          <Label htmlFor="register-firstname">Prénom</Label>
          <Input
            id="register-firstname"
            type="text"
            placeholder="Prénom"
            value={firstname}
            onChange={(event) => onFirstnameChange(event.target.value)}
          />
        </div>
        <div className="flex flex-col gap-2">
          <Label htmlFor="register-lastname">Nom</Label>
          <Input
            id="register-lastname"
            type="text"
            placeholder="Nom"
            value={lastname}
            onChange={(event) => onLastnameChange(event.target.value)}
          />
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <Label htmlFor="register-email">Email</Label>
        <Input
          id="register-email"
          type="email"
          placeholder="vous@exemple.fr"
          value={email}
          onChange={(event) => onEmailChange(event.target.value)}
          required
          autoComplete="email"
          autoFocus
        />
      </div>

      <div className="grid grid-cols-2 gap-3">
        <div className="flex flex-col gap-2">
          <Label htmlFor="register-username">Nom d&apos;utilisateur</Label>
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
          <Label htmlFor="register-birthdate">Date de naissance</Label>
          <Input
            id="register-birthdate"
            type="date"
            min={minBirthdate}
            max={maxBirthdate}
            value={birthdate}
            onChange={(event) => onBirthdateChange(event.target.value)}
            required
          />
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <Label htmlFor="register-password">Mot de passe</Label>
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
        Continuer
        <ArrowRightIcon data-icon="inline-end" />
      </Button>
    </form>
  );
}
