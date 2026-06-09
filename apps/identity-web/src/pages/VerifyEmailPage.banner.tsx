import { useEffect, useRef } from 'react';
import { CheckCircle2Icon, CircleAlertIcon, MailCheckIcon } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import type { VerificationStatus } from './VerifyEmailPage.shared';

export function VerifyEmailStatusBanner({
  status,
  message,
}: {
  status: VerificationStatus;
  message: string;
}) {
  const bannerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const element = bannerRef.current;
    if (!element) {
      return;
    }

    element.classList.remove('nvbes-banner-enter');
    void element.offsetWidth;
    element.classList.add('nvbes-banner-enter');
  }, [message]);

  const Icon =
    status === 'verified' || status === 'sent'
      ? CheckCircle2Icon
      : status === 'error'
        ? CircleAlertIcon
        : MailCheckIcon;

  return (
    <>
      <style>{`
        @keyframes nvbes-banner-slide-in {
          from { opacity: 0; transform: translateY(-6px) scale(0.98); }
          to { opacity: 1; transform: translateY(0) scale(1); }
        }
        @keyframes nvbes-icon-pop {
          0% { transform: scale(0); opacity: 0; }
          60% { transform: scale(1.15); }
          100% { transform: scale(1); opacity: 1; }
        }
        .nvbes-banner-enter {
          animation: nvbes-banner-slide-in 0.35s cubic-bezier(0.16, 1, 0.3, 1) both;
        }
        .nvbes-icon-pop {
          animation: nvbes-icon-pop 0.4s cubic-bezier(0.16, 1, 0.3, 1) both;
        }
      `}</style>
      <Alert
        ref={bannerRef}
        variant={status === 'error' ? 'destructive' : 'default'}
        className="nvbes-banner-enter"
      >
        <Icon className="nvbes-icon-pop" />
        <AlertDescription>{message}</AlertDescription>
      </Alert>
    </>
  );
}
