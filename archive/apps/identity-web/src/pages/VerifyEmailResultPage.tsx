import { VerifyEmailResultView } from './VerifyEmailResultPage.shared';
import { useVerifyEmailResultPage } from './useVerifyEmailResultPage';

export default function VerifyEmailResultPage() {
  const { navigate, result } = useVerifyEmailResultPage();

  return (
    <VerifyEmailResultView
      result={result}
      onGoLogin={() => void navigate({ to: '/login' })}
      onGoVerify={() => void navigate({ to: '/verify' })}
    />
  );
}
