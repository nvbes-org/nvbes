import { useEffect, useRef } from 'react';
import { CheckCircle2Icon, CircleAlertIcon, MailCheckIcon } from 'lucide-react';
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

  const colors =
    status === 'verified' || status === 'sent'
      ? {
          border: 'border-emerald-500/30',
          bg: 'bg-emerald-500/5',
          text: 'text-emerald-700',
          icon: 'text-emerald-500',
        }
      : status === 'error'
        ? {
            border: 'border-red-500/30',
            bg: 'bg-red-500/5',
            text: 'text-red-700',
            icon: 'text-red-500',
          }
        : {
            border: 'border-blue-500/30',
            bg: 'bg-blue-500/5',
            text: 'text-blue-700',
            icon: 'text-blue-500',
          };

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
      <div
        ref={bannerRef}
        className={`nvbes-banner-enter flex items-start gap-3 rounded-lg border ${colors.border} ${colors.bg} px-4 py-3 text-sm`}
      >
        <Icon className={`nvbes-icon-pop mt-0.5 size-4 shrink-0 ${colors.icon}`} />
        <p className={colors.text}>{message}</p>
      </div>
    </>
  );
}
