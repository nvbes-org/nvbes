import { KeyRoundIcon, type LucideIcon, MailIcon, ShieldCheckIcon } from 'lucide-react';
import { cn } from '@/lib/classnames';

export type LoginStep = 'chooser' | 'identifier' | 'password' | 'webauthn' | 'mfa' | 'consent';

const STEPS: {
  key: 'identifier' | 'password' | 'mfa';
  label: string;
  icon: LucideIcon;
}[] = [
  { key: 'identifier', label: 'Identification', icon: MailIcon },
  { key: 'password', label: 'Authentification', icon: KeyRoundIcon },
  { key: 'mfa', label: 'Vérification', icon: ShieldCheckIcon },
];

export function LoginProgress({ isOAuthFlow, step }: { isOAuthFlow: boolean; step: LoginStep }) {
  if (step === 'consent' || step === 'chooser') {
    return null;
  }
  const steps = isOAuthFlow ? STEPS : STEPS.filter((s) => s.key !== 'mfa');
  const progressStep = step === 'webauthn' || (!isOAuthFlow && step === 'mfa') ? 'password' : step;
  const stepIndex = steps.findIndex((s) => s.key === progressStep);

  return (
    <div className="mb-8" aria-label={`Connexion en ${steps.length} étapes`}>
      <div className="flex items-center gap-1.5">
        {steps.map((s, i) => {
          const active = progressStep === s.key;
          const complete =
            (s.key === 'identifier' && (progressStep === 'password' || progressStep === 'mfa')) ||
            (s.key === 'password' && progressStep === 'mfa');
          const Icon = s.icon;
          return (
            <div key={s.key} className="flex items-center gap-1.5">
              <div
                className={cn(
                  'flex size-7 items-center justify-center rounded-full border text-xs transition-all duration-500',
                  active && 'border-primary bg-primary text-primary-foreground shadow-sm',
                  complete && 'border-primary/50 bg-primary/10 text-primary',
                  !active && !complete && 'border-border text-muted-foreground/50',
                )}
              >
                <Icon className="size-3.5" />
              </div>
              {i < steps.length - 1 && (
                <div
                  className={cn(
                    'h-px w-6 transition-colors duration-500 sm:w-8',
                    complete ? 'bg-primary/50' : 'bg-border',
                  )}
                />
              )}
            </div>
          );
        })}
      </div>
      {stepIndex >= 0 && (
        <p className="mt-2 text-xs font-medium text-muted-foreground">{steps[stepIndex].label}</p>
      )}
    </div>
  );
}
