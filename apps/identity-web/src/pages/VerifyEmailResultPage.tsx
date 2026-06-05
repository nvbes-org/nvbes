import { useLocation, useNavigate } from '@tanstack/react-router';
import {
  CheckCircle2Icon,
  CircleAlertIcon,
  ClockIcon,
  Loader2Icon,
  ShieldAlertIcon,
} from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardDescription, CardFooter, CardHeader, CardTitle } from '@/components/ui/card';

type ResultState =
  | { kind: 'verifying' }
  | { kind: 'success'; email: string }
  | { kind: 'token_expired' }
  | { kind: 'token_invalid' }
  | { kind: 'error'; message: string };

interface ApiError {
  error?: {
    code?: string;
    message?: string;
  };
}

function AnimatedIcon({ children }: { children: React.ReactNode }) {
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.classList.remove('nvbes-verify-icon-enter');
    void el.offsetWidth;
    el.classList.add('nvbes-verify-icon-enter');
  }, []);

  return (
    <>
      <style>{`
        @keyframes nvbes-verify-icon-pop {
          0% { transform: scale(0); opacity: 0; }
          60% { transform: scale(1.15); }
          100% { transform: scale(1); opacity: 1; }
        }
        .nvbes-verify-icon-enter {
          animation: nvbes-verify-icon-pop 0.4s cubic-bezier(0.16, 1, 0.3, 1) both;
        }
      `}</style>
      <div ref={ref} className="nvbes-verify-icon-enter">
        {children}
      </div>
    </>
  );
}

export default function VerifyEmailResultPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const searchParams = new URLSearchParams(location.searchStr);
  const token = searchParams.get('token');
  const [result, setResult] = useState<ResultState>({ kind: 'verifying' });

  useEffect(() => {
    if (!token) {
      setResult({ kind: 'token_invalid' });
      return;
    }

    let cancelled = false;

    const verify = async () => {
      const csrfMatch =
        typeof document !== 'undefined'
          ? document.cookie.match(/(?:^|;\s*)csrf_token=([^;]*)/)
          : null;
      const csrfToken = csrfMatch?.[1];
      const headers: Record<string, string> = {
        'Content-Type': 'application/json',
      };
      if (csrfToken) {
        headers['X-CSRF-Token'] = csrfToken;
      }

      try {
        const response = await fetch('/auth/verify-email', {
          method: 'POST',
          headers,
          body: JSON.stringify({ token }),
          credentials: 'include',
        });

        if (cancelled) return;

        if (response.ok) {
          const data = await response.json();
          setResult({ kind: 'success', email: data.user?.email ?? '' });
          return;
        }

        const body = (await response.json().catch(() => ({}))) as ApiError;
        const code = body.error?.code ?? '';
        const message = body.error?.message ?? '';

        if (code === 'verification_token_expired' || message.toLowerCase().includes('expired')) {
          setResult({ kind: 'token_expired' });
        } else if (
          code === 'verification_token_not_found' ||
          message.toLowerCase().includes('invalid')
        ) {
          setResult({ kind: 'token_invalid' });
        } else {
          setResult({
            kind: 'error',
            message: message || 'Échec de la vérification.',
          });
        }
      } catch (err) {
        console.error('Email verification failed:', err);
        if (!cancelled) {
          setResult({
            kind: 'error',
            message: 'Impossible de contacter le serveur.',
          });
        }
      }
    };

    void verify();

    return () => {
      cancelled = true;
    };
  }, [token]);

  return (
    <div className="flex min-h-screen items-center justify-center bg-background px-4 py-10">
      <Card className="w-full max-w-sm">
        {result.kind === 'verifying' ? (
          <CardHeader className="text-center pb-2">
            <div className="mx-auto mb-3">
              <Loader2Icon className="size-10 animate-spin text-muted-foreground" />
            </div>
            <CardTitle>Vérification en cours</CardTitle>
            <CardDescription>Nous vérifions ton adresse email...</CardDescription>
          </CardHeader>
        ) : result.kind === 'success' ? (
          <>
            <CardHeader className="text-center pb-2">
              <div className="mx-auto mb-3">
                <AnimatedIcon>
                  <div className="flex size-14 items-center justify-center rounded-full bg-emerald-500/10 ring-1 ring-emerald-500/20">
                    <CheckCircle2Icon className="size-7 text-emerald-500" />
                  </div>
                </AnimatedIcon>
              </div>
              <CardTitle>Email vérifié</CardTitle>
              <CardDescription className="leading-relaxed">
                {result.email ? (
                  <>
                    Ton adresse <span className="font-medium text-foreground">{result.email}</span>{' '}
                    est confirmée. Tu peux maintenant te connecter.
                  </>
                ) : (
                  'Ton adresse email est confirmée. Tu peux maintenant te connecter.'
                )}
              </CardDescription>
            </CardHeader>
            <CardFooter className="flex-col gap-3 pt-2">
              <Button className="w-full" onClick={() => void navigate({ to: '/login' })}>
                Se connecter
              </Button>
            </CardFooter>
          </>
        ) : result.kind === 'token_expired' ? (
          <>
            <CardHeader className="text-center pb-2">
              <div className="mx-auto mb-3">
                <AnimatedIcon>
                  <div className="flex size-14 items-center justify-center rounded-full bg-amber-500/10 ring-1 ring-amber-500/20">
                    <ClockIcon className="size-7 text-amber-600" />
                  </div>
                </AnimatedIcon>
              </div>
              <CardTitle>Lien expiré</CardTitle>
              <CardDescription>
                Ce lien de vérification a expiré. Demande un nouveau lien pour vérifier ton adresse
                email.
              </CardDescription>
            </CardHeader>
            <CardFooter className="flex-col gap-3 pt-2">
              <Button className="w-full" onClick={() => void navigate({ to: '/verify' })}>
                Renvoyer le lien
              </Button>
              <Button
                variant="outline"
                className="w-full"
                onClick={() => void navigate({ to: '/login' })}
              >
                Retour à la connexion
              </Button>
            </CardFooter>
          </>
        ) : result.kind === 'token_invalid' ? (
          <>
            <CardHeader className="text-center pb-2">
              <div className="mx-auto mb-3">
                <AnimatedIcon>
                  <div className="flex size-14 items-center justify-center rounded-full bg-red-500/10 ring-1 ring-red-500/20">
                    <CircleAlertIcon className="size-7 text-red-500" />
                  </div>
                </AnimatedIcon>
              </div>
              <CardTitle>Lien invalide</CardTitle>
              <CardDescription>
                Ce lien de vérification n'est pas valide. Il a peut-être déjà été utilisé ou est
                incorrect.
              </CardDescription>
            </CardHeader>
            <CardFooter className="flex-col gap-3 pt-2">
              <Button className="w-full" onClick={() => void navigate({ to: '/verify' })}>
                Demander un nouveau lien
              </Button>
              <Button
                variant="outline"
                className="w-full"
                onClick={() => void navigate({ to: '/login' })}
              >
                Retour à la connexion
              </Button>
            </CardFooter>
          </>
        ) : (
          <>
            <CardHeader className="text-center pb-2">
              <div className="mx-auto mb-3">
                <AnimatedIcon>
                  <div className="flex size-14 items-center justify-center rounded-full bg-red-500/10 ring-1 ring-red-500/20">
                    <ShieldAlertIcon className="size-7 text-red-500" />
                  </div>
                </AnimatedIcon>
              </div>
              <CardTitle>Erreur</CardTitle>
              <CardDescription>{result.message}</CardDescription>
            </CardHeader>
            <CardFooter className="flex-col gap-3 pt-2">
              <Button className="w-full" onClick={() => void navigate({ to: '/verify' })}>
                Réessayer
              </Button>
              <Button
                variant="outline"
                className="w-full"
                onClick={() => void navigate({ to: '/login' })}
              >
                Retour à la connexion
              </Button>
            </CardFooter>
          </>
        )}
      </Card>
    </div>
  );
}
