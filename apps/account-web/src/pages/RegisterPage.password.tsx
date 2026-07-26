import { useDeferredValue, useEffect, useState } from 'react';

import { cn } from '@/lib/utils';

interface PasswordStrength {
  feedback: {
    warning: string | null;
  };
  score: number;
}

const strengthLevels = [
  {
    label: 'Très faible',
    color: 'bg-destructive',
    textColor: 'text-destructive',
  },
  {
    label: 'Très faible',
    color: 'bg-orange-500',
    textColor: 'text-orange-600',
  },
  {
    label: 'Faible',
    color: 'bg-amber-500',
    textColor: 'text-amber-600',
  },
  {
    label: 'Bon',
    color: 'bg-teal-600',
    textColor: 'text-teal-700',
  },
  {
    label: 'Fort',
    color: 'bg-emerald-600',
    textColor: 'text-emerald-700',
  },
] as const;

export function PasswordStrengthMeter({ password }: { password: string }) {
  const deferredPassword = useDeferredValue(password);
  const [result, setResult] = useState<PasswordStrength | null>(null);

  useEffect(() => {
    if (!deferredPassword) {
      setResult(null);
      return;
    }

    let active = true;
    void import('./RegisterPage.password-strength').then(({ estimatePasswordStrength }) => {
      if (active) {
        setResult(estimatePasswordStrength(deferredPassword));
      }
    });

    return () => {
      active = false;
    };
  }, [deferredPassword]);

  if (!result) {
    return null;
  }

  const score = Math.min(Math.max(result.score, 0), strengthLevels.length - 1);
  const level = strengthLevels[score] ?? strengthLevels[0];

  return (
    <div className="flex flex-col gap-1.5">
      <div
        className="grid h-1.5 grid-cols-5 gap-1"
        aria-label={`Robustesse du mot de passe : ${level.label}`}
        aria-valuemin={0}
        aria-valuemax={4}
        aria-valuenow={score}
        role="meter"
      >
        {strengthLevels.map((strengthLevel, index) => (
          <span
            key={strengthLevel.color}
            className={cn(
              'h-full rounded-full bg-muted transition-colors',
              index <= score && level.color,
            )}
            aria-hidden="true"
          />
        ))}
      </div>
      <div className="flex items-center justify-between">
        <span className={cn('text-xs font-medium', level.textColor)}>{level.label}</span>
        {result.feedback.warning && (
          <span className="max-w-[200px] truncate text-xs text-destructive">
            {result.feedback.warning}
          </span>
        )}
      </div>
    </div>
  );
}
