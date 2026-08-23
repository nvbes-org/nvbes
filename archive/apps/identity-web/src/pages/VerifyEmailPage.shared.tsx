export { VerifyEmailStatusBanner } from './VerifyEmailPage.banner';
export { VerifyEmailCard } from './VerifyEmailPage.card';

export type VerificationStatus = 'pending' | 'verifying' | 'verified' | 'error' | 'sent';

export type VerificationLocationState = {
  accountName?: string | null;
  email?: string | null;
  resendAvailableAt?: string | null;
};

export function sameEmail(left: string, right: string): boolean {
  return left.trim().toLowerCase() === right.trim().toLowerCase();
}
