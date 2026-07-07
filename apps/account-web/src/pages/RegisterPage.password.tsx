import { zxcvbn, zxcvbnOptions } from '@zxcvbn-ts/core';
import { adjacencyGraphs, dictionary as commonDictionary } from '@zxcvbn-ts/language-common';
import { dictionary as frDictionary, translations } from '@zxcvbn-ts/language-fr';
import { useMemo } from 'react';

import { cn } from '@/lib/utils';

zxcvbnOptions.setOptions({
  dictionary: {
    ...commonDictionary,
    ...frDictionary,
  },
  graphs: adjacencyGraphs,
  translations,
});

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
  const result = useMemo(() => {
    if (!password) {
      return null;
    }

    return zxcvbn(password);
  }, [password]);

  if (!result) {
    return null;
  }

  const level = strengthLevels[result.score] ?? strengthLevels[0];

  return (
    <div className="flex flex-col gap-1.5">
      <div
        className="grid h-1.5 grid-cols-5 gap-1"
        aria-label={`Robustesse du mot de passe : ${level.label}`}
        aria-valuemin={0}
        aria-valuemax={4}
        aria-valuenow={result.score}
        role="meter"
      >
        {strengthLevels.map((strengthLevel, index) => (
          <span
            key={strengthLevel.color}
            className={cn(
              'h-full rounded-full bg-muted transition-colors',
              index <= result.score && level.color,
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
