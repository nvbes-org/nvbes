import { useVerifyEmailPageAction } from './useVerifyEmailPage.action';
import { useVerifyEmailPageEffects } from './useVerifyEmailPage.effects';
import { useVerifyEmailPageState } from './useVerifyEmailPage.state';

export function useVerifyEmailPage() {
  const state = useVerifyEmailPageState();

  useVerifyEmailPageEffects({
    token: state.token,
    verified: state.verified,
    setVerified: state.setVerified,
    setStatus: state.setStatus,
    setMessage: state.setMessage,
    setNow: state.setNow,
  });

  const action = useVerifyEmailPageAction({
    originalEmail: state.originalEmail,
    setOriginalEmail: state.setOriginalEmail,
    emailDraft: state.emailDraft,
    resendAvailableAt: state.resendAvailableAt,
    now: state.now,
    working: state.working,
    verified: state.verified,
    setWorking: state.setWorking,
    setMessage: state.setMessage,
    setStatus: state.setStatus,
    setVerified: state.setVerified,
    setResendAvailableAt: state.setResendAvailableAt,
  });

  return {
    accountName: state.accountName,
    canSend: action.canSend,
    emailChanged: action.emailChanged,
    emailDraft: state.emailDraft,
    emailLocked: state.verified,
    handleAction: action.handleAction,
    message: state.message,
    navigate: state.navigate,
    resendInSeconds: action.resendInSeconds,
    setEmailDraft: state.setEmailDraft,
    status: state.status,
    verified: state.verified,
    working: state.working,
  };
}
