import type { ReactNode } from 'react';

import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import type { LoginStep } from './LoginProgress';

export function LoginPageLoading() {
  return (
    <div className="flex flex-1 items-center justify-center bg-muted/30">
      <div className="size-6 animate-spin rounded-full border-2 border-primary border-t-transparent" />
    </div>
  );
}

export function LoginPageMobileBrand() {
  return (
    <div className="mb-8 flex items-center gap-2.5 lg:hidden">
      <div className="inline-flex size-8 items-center justify-center rounded-lg bg-primary/15">
        <div className="size-3 rounded-sm bg-primary" />
      </div>
      <span className="text-lg font-semibold tracking-tight text-foreground/85">nvbes</span>
    </div>
  );
}

export function LoginPageError({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {message}
    </div>
  );
}

function titleForStep(step: LoginStep) {
  if (step === 'consent') return "Demande d'autorisation";
  if (step === 'chooser') return 'Choisir un compte';
  return 'Connexion';
}

function descriptionForStep(step: LoginStep) {
  if (step === 'identifier') return 'Entrez votre email pour commencer.';
  if (step === 'password') return 'Saisissez votre mot de passe.';
  if (step === 'consent') return "L'application souhaite accéder à votre compte.";
  if (step === 'chooser') return 'pour continuer sur nvbes';
  return 'Vérification en deux étapes.';
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
