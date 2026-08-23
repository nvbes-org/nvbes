import {
  VerifyEmailErrorState,
  VerifyEmailExpiredState,
  VerifyEmailInvalidState,
  VerifyEmailSuccessState,
  VerifyEmailVerifyingState,
} from './VerifyEmailResultPage.states';

export type VerifyEmailResultState =
  | { kind: 'verifying' }
  | { kind: 'success'; email: string }
  | { kind: 'token_expired' }
  | { kind: 'token_invalid' }
  | { kind: 'error'; message: string };

export interface VerifyEmailApiError {
  error?: {
    code?: string;
    message?: string;
  };
}

export function VerifyEmailResultView({
  result,
  onGoLogin,
  onGoVerify,
}: {
  result: VerifyEmailResultState;
  onGoLogin: () => void;
  onGoVerify: () => void;
}) {
  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4 py-10">
      {result.kind === 'verifying' ? (
        <VerifyEmailVerifyingState />
      ) : result.kind === 'success' ? (
        <VerifyEmailSuccessState result={result} onGoLogin={onGoLogin} />
      ) : result.kind === 'token_expired' ? (
        <VerifyEmailExpiredState onGoLogin={onGoLogin} onGoVerify={onGoVerify} />
      ) : result.kind === 'token_invalid' ? (
        <VerifyEmailInvalidState onGoLogin={onGoLogin} onGoVerify={onGoVerify} />
      ) : (
        <VerifyEmailErrorState
          message={result.message}
          onGoLogin={onGoLogin}
          onGoVerify={onGoVerify}
        />
      )}
    </div>
  );
}
