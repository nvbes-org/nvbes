import { changeVerificationEmail, resendVerificationEmail } from '../identity.email-verification';
import { sameEmail } from './VerifyEmailPage.shared';

export function useVerifyEmailPageAction({
  originalEmail,
  setOriginalEmail,
  emailDraft,
  resendAvailableAt,
  now,
  working,
  verified,
  setWorking,
  setMessage,
  setStatus,
  setVerified,
  setResendAvailableAt,
}: {
  originalEmail: string;
  setOriginalEmail: React.Dispatch<React.SetStateAction<string>>;
  emailDraft: string;
  resendAvailableAt: Date | null;
  now: Date;
  working: boolean;
  verified: boolean;
  setWorking: React.Dispatch<React.SetStateAction<boolean>>;
  setMessage: React.Dispatch<React.SetStateAction<string | null>>;
  setStatus: React.Dispatch<
    React.SetStateAction<'pending' | 'verifying' | 'sent' | 'verified' | 'error'>
  >;
  setVerified: React.Dispatch<React.SetStateAction<boolean>>;
  setResendAvailableAt: React.Dispatch<React.SetStateAction<Date | null>>;
}) {
  const draft = emailDraft.trim();
  const hasEmail = draft.length > 0;
  const emailChanged = originalEmail.length > 0 && hasEmail && !sameEmail(draft, originalEmail);
  const resendInSeconds = resendAvailableAt
    ? Math.max(0, Math.ceil((resendAvailableAt.getTime() - now.getTime()) / 1000))
    : 0;
  const canSend = !verified && hasEmail && !working && (emailChanged || resendInSeconds === 0);

  const handleAction = async () => {
    if (!hasEmail) {
      return;
    }

    setWorking(true);
    setMessage(null);

    try {
      const result = emailChanged
        ? await changeVerificationEmail(originalEmail, draft)
        : await resendVerificationEmail(draft);

      setResendAvailableAt(
        result.verification_resend_available_at
          ? new Date(result.verification_resend_available_at)
          : null,
      );

      if (result.email_verified) {
        setVerified(true);
        setStatus('verified');
        setMessage('Compte vérifié.');
        return;
      }

      setStatus('sent');
      if (emailChanged) {
        setMessage('Adresse mise à jour. Vérification renvoyée.');
        setOriginalEmail(draft);
      } else {
        setMessage('Vérification renvoyée.');
      }
    } catch (error) {
      setStatus('error');
      setMessage(error instanceof Error ? error.message : "Échec de l'envoi.");
    } finally {
      setWorking(false);
    }
  };

  return {
    canSend,
    emailChanged,
    resendInSeconds,
    handleAction,
  };
}
