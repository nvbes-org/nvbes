import { VerifyEmailCard } from './VerifyEmailPage.shared';
import { useVerifyEmailPage } from './useVerifyEmailPage';

export default function VerifyEmailPage() {
  const {
    canSend,
    emailChanged,
    emailDraft,
    emailLocked,
    handleAction,
    message,
    navigate,
    resendInSeconds,
    setEmailDraft,
    status,
    verified,
    working,
  } = useVerifyEmailPage();

  return (
    <VerifyEmailCard
      verified={verified}
      message={message}
      status={status}
      emailDraft={emailDraft}
      emailLocked={emailLocked}
      canSend={canSend}
      working={working}
      emailChanged={emailChanged}
      resendInSeconds={resendInSeconds}
      onEmailDraftChange={setEmailDraft}
      onAction={handleAction}
      onBackToLogin={() => void navigate({ to: '/login' })}
    />
  );
}
