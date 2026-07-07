import type { ReactNode } from 'react';

import { Alert, AlertDescription } from '@/components/ui/alert';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Spinner } from '@/components/ui/spinner';
import type { LoginStep } from './LoginProgress';

export function LoginPageLoading() {
  return (
    <div className="flex flex-1 items-center justify-center bg-muted/30">
      <Spinner className="size-6 text-primary" />
    </div>
  );
}

export function LoginPageError({ message }: { message: string }) {
  return (
    <Alert variant="destructive">
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  );
}

function titleForStep(step: LoginStep) {
  if (step === 'consent') return "Demande d'autorisation";
  if (step === 'chooser') return 'Choisir un compte';
  return 'nvbes';
}

function descriptionForStep(step: LoginStep) {
  if (step === 'identifier') return 'Connectez-vous à votre espace sécurisé nvbes.';
  if (step === 'password') return 'Confirmez votre identité pour continuer.';
  if (step === 'webauthn') return 'Validez la connexion avec votre clé de sécurité.';
  if (step === 'consent') return "L'application souhaite accéder à votre compte.";
  if (step === 'chooser') return 'pour continuer sur nvbes';
  return 'Terminez la vérification de sécurité.';
}

export function LoginPageCard({
  step,
  transitionDirection = 'forward',
  footer,
  children,
}: {
  step: LoginStep;
  transitionDirection?: 'forward' | 'backward';
  footer?: ReactNode;
  children: ReactNode;
}) {
  return (
    <Card
      className={
        transitionDirection === 'backward'
          ? 'animate-login-card-enter-backward motion-reduce:animate-none'
          : 'animate-login-card-enter-forward motion-reduce:animate-none'
      }
    >
      <CardHeader>
        <CardTitle>{titleForStep(step)}</CardTitle>
        <CardDescription>{descriptionForStep(step)}</CardDescription>
      </CardHeader>
      <CardContent>{children}</CardContent>
      <CardFooter className="justify-center">{footer}</CardFooter>
    </Card>
  );
}
