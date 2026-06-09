import { zxcvbn, zxcvbnOptions } from '@zxcvbn-ts/core';
import { adjacencyGraphs, dictionary as commonDictionary } from '@zxcvbn-ts/language-common';
import { dictionary as frDictionary, translations } from '@zxcvbn-ts/language-fr';
import { useMemo } from 'react';

import { Progress } from '@/components/ui/progress';

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

  return (
    <div className="flex flex-col gap-1.5">
      <Progress value={(result.score / 4) * 100} />
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
