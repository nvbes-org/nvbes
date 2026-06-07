import { CheckCircle2Icon } from 'lucide-react';
import { ResultPrimaryAction } from './VerifyEmailResultPage.actions';
import { ResultCard } from './VerifyEmailResultPage.card';
import type { VerifyEmailResultState } from './VerifyEmailResultPage.shared';
import { ResultStatusIcon } from './VerifyEmailResultPage.status_icon';

export function VerifyEmailSuccessState({
  result,
  onGoLogin,
}: {
  result: Extract<VerifyEmailResultState, { kind: 'success' }>;
  onGoLogin: () => void;
}) {
  return (
    <ResultCard
      title="Email vérifié"
      description={
        result.email ? (
          <>
            Ton adresse <span className="font-medium text-foreground">{result.email}</span> est
            confirmée. Tu peux maintenant te connecter.
          </>
        ) : (
          'Ton adresse email est confirmée. Tu peux maintenant te connecter.'
        )
      }
      icon={
        <ResultStatusIcon tone="emerald">
          <CheckCircle2Icon className="size-7 text-emerald-500" />
        </ResultStatusIcon>
      }
      actions={<ResultPrimaryAction label="Se connecter" onClick={onGoLogin} />}
    />
  );
}
