import { useLocation, useNavigate } from '@tanstack/react-router';
import { CheckCircle2Icon, CircleAlertIcon, MailCheckIcon } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  changeVerificationEmail,
  resendVerificationEmail,
  verifyEmailToken,
} from '../identity.email-verification';

type VerificationLocationState = {
  email?: string | null;
  resendAvailableAt?: string | null;
};

type VerificationStatus = 'pending' | 'verifying' | 'verified' | 'error' | 'sent';

function sameEmail(left: string, right: string): boolean {
  return left.trim().toLowerCase() === right.trim().toLowerCase();
}

function StatusBanner({ status, message }: { status: VerificationStatus; message: string }) {
  const bannerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = bannerRef.current;
    if (!el) return;
    el.classList.remove('nvbes-banner-enter');
    void el.offsetWidth;
    el.classList.add('nvbes-banner-enter');
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
        .nvbes-banner-enter .nvbes-icon-pop {
          animation-delay: 0.05s;
        }
        .nvbes-banner-enter .nvbes-banner-text {
          animation-delay: 0.1s;
        }
      `}</style>
      <div
        ref={bannerRef}
        className={`nvbes-banner-enter flex items-start gap-3 rounded-lg border ${colors.border} ${colors.bg} px-4 py-3 text-sm`}
      >
        <Icon className={`size-4 shrink-0 mt-0.5 ${colors.icon} nvbes-icon-pop`} />
        <p className={`${colors.text}`}>{message}</p>
      </div>
    </>
  );
}

export default function VerifyEmailPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const searchParams = new URLSearchParams(location.searchStr);
  const locationState = location.state as VerificationLocationState | null;

  const emailFromState = locationState?.email ?? '';
  const token = searchParams.get('token') ?? null;
  const [status, setStatus] = useState<VerificationStatus>(token ? 'verifying' : 'pending');
  const [message, setMessage] = useState<string | null>(null);
  const [originalEmail, setOriginalEmail] = useState(emailFromState);
  const [emailDraft, setEmailDraft] = useState(emailFromState);
  const [resendAvailableAt, setResendAvailableAt] = useState<Date | null>(
    locationState?.resendAvailableAt ? new Date(locationState.resendAvailableAt) : null,
  );
  const [now, setNow] = useState(() => new Date());
  const [working, setWorking] = useState(false);
  const [verified, setVerified] = useState(false);

  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(new Date());
    }, 1000);

    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (!token || verified) {
      return;
    }

    let cancelled = false;
    const run = async () => {
      try {
        const result = await verifyEmailToken(token);
        if (cancelled) {
          return;
        }

        if (result.success) {
          setStatus('verified');
          setMessage('Compte vérifié.');
          setVerified(true);
        } else {
          setStatus('error');
          setMessage('La vérification a échoué.');
        }
      } catch (error) {
        if (!cancelled) {
          setStatus('error');
          setMessage(error instanceof Error ? error.message : 'Échec de la vérification.');
        }
      }
    };

    void run();

    return () => {
      cancelled = true;
    };
  }, [token, verified]);

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

  const emailLocked = verified;

  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4 py-10 sm:px-6">
      <Card className="w-full max-w-md">
        <CardHeader>
          <CardTitle>Vérifie ton email</CardTitle>
          <CardDescription>
            {verified ? 'Compte vérifié.' : 'Confirme ton adresse email.'}
          </CardDescription>
        </CardHeader>

        <CardContent className="flex flex-col gap-5">
          {message ? <StatusBanner status={status} message={message} /> : null}

          <div className="flex flex-col gap-3">
            <Label htmlFor="verification-email">Email du compte</Label>
            <Input
              id="verification-email"
              type="email"
              value={emailDraft}
              onChange={(event) => setEmailDraft(event.target.value)}
              autoComplete="email"
              disabled={emailLocked}
              placeholder="adresse@email.com"
            />
          </div>
        </CardContent>

        <CardFooter className="flex flex-col gap-3 sm:flex-row">
          <Button
            type="button"
            variant="outline"
            className="w-full sm:flex-1"
            onClick={() => void navigate({ to: '/login' })}
          >
            Retour à la connexion
          </Button>
          {!verified ? (
            <Button
              type="button"
              className="w-full sm:flex-1"
              onClick={handleAction}
              disabled={!canSend}
            >
              {working
                ? 'Envoi...'
                : emailChanged
                  ? 'Mettre à jour et renvoyer'
                  : resendInSeconds > 0
                    ? `Renvoyer dans ${resendInSeconds}s`
                    : 'Renvoyer la vérification'}
            </Button>
          ) : (
            <Button type="button" className="w-full sm:flex-1" disabled>
              Vérifié
            </Button>
          )}
        </CardFooter>
      </Card>
    </div>
  );
}
