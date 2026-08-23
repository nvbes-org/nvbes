import { AuthBrandPanel } from '@/components/AuthBrandPanel';
import type { LoginStep } from './LoginProgress';

const STEP_COPY: Record<LoginStep, { title: string; description: string }> = {
  chooser: {
    title: 'Choisir un compte',
    description: 'Sélectionnez le compte avec lequel vous souhaitez continuer.',
  },
  identifier: {
    title: 'Se connecter',
    description: 'Utilisez votre compte nvbes pour accéder à votre espace sécurisé.',
  },
  password: {
    title: 'Bienvenue',
    description: 'Confirmez votre identité pour continuer vers vos services nvbes.',
  },
  webauthn: {
    title: 'Clé de sécurité',
    description: 'Validez la connexion avec votre clé de sécurité.',
  },
  mfa: {
    title: 'Vérification',
    description: 'Terminez la vérification de sécurité de votre compte.',
  },
  consent: {
    title: "Demande d'autorisation",
    description: "Vérifiez les accès demandés par l'application.",
  },
};

export function LoginBrandPanel({ step }: { step: LoginStep }) {
  const copy = STEP_COPY[step];

  return <AuthBrandPanel title={copy.title} description={copy.description} />;
}
