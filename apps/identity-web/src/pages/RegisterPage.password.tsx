import { zxcvbn, zxcvbnOptions } from '@zxcvbn-ts/core';
import { adjacencyGraphs, dictionary as commonDictionary } from '@zxcvbn-ts/language-common';
import { dictionary as frDictionary, translations } from '@zxcvbn-ts/language-fr';
import { useMemo } from 'react';

import { cn } from '@/lib/classnames';

zxcvbnOptions.setOptions({
  dictionary: {
    ...commonDictionary,
    ...frDictionary,
  },
  graphs: adjacencyGraphs,
  translations,
});

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

  const bars = [
    { score: 1, label: 'très faible' },
    { score: 2, label: 'faible' },
    { score: 3, label: 'bon' },
    { score: 4, label: 'fort' },
  ];

  const getBarColor = (barScore: number) => {
    if (result.score >= barScore) {
      if (result.score <= 1) {
        return 'bg-destructive';
      }
      if (result.score === 2) {
        return 'bg-orange-500';
      }
      if (result.score === 3) {
        return 'bg-blue-500';
      }
      return 'bg-emerald-500';
    }

    return 'bg-muted';
  };

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex h-1.5 gap-1">
        {bars.map((bar) => (
          <div
            key={bar.score}
            className={cn(
              'flex-1 rounded-full transition-colors duration-300',
              getBarColor(bar.score),
            )}
          />
        ))}
      </div>
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">
          {result.score <= 1
            ? 'Très faible'
            : result.score === 2
              ? 'Faible'
              : result.score === 3
                ? 'Bon'
                : 'Fort'}
        </span>
        {result.feedback.warning && (
          <span className="max-w-[200px] truncate text-xs text-destructive">
            {result.feedback.warning}
          </span>
        )}
      </div>
    </div>
  );
}
