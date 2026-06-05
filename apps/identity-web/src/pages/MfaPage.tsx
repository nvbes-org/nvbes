import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import {
  completeWebAuthnStepUp,
  listMfaFactors,
  MfaError,
  removeMfaFactor,
  stepUp,
  WebauthnBrowserError,
} from '@nvbes/identity-sdk-web';
import { useNavigate } from '@tanstack/react-router';
import { ArrowLeft, Fingerprint, KeyRound, ShieldAlert, Smartphone, Trash2 } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';

const FACTOR_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  totp: Smartphone,
  webauthn: KeyRound,
  recovery: ShieldAlert,
};

const FACTOR_ADD_ACTIONS = [
  {
    icon: Smartphone,
    label: "Code d'authentification (TOTP)",
    path: '/account/mfa/totp/setup',
  },
  {
    icon: Fingerprint,
    label: 'Passkey',
    path: '/account/mfa/passkey/setup',
  },
  {
    icon: KeyRound,
    label: 'Clé de sécurité',
    path: '/account/mfa/security-key/setup',
  },
  {
    icon: ShieldAlert,
    label: 'Codes de récupération',
    path: '/account/mfa/recovery-codes',
  },
] as const;

type StepUpMethod = 'password' | 'totp' | 'webauthn' | 'recovery';
const WEBAUTHN_TIMEOUT_MS = 60_000;

function getFactorTypeIcon(type: string): React.ComponentType<{ className?: string }> {
  return FACTOR_ICONS[type] ?? Smartphone;
}

function factorLabel(factor: MfaFactorView) {
  if (factor.factor_type === 'webauthn' && factor.kind === 'passkey') {
    return 'Passkey';
  }
  if (factor.factor_type === 'webauthn' && factor.kind === 'security_key') {
    return 'Clé de sécurité';
  }

  switch (factor.factor_type) {
    case 'totp':
      return "Code d'authentification (TOTP)";
    case 'webauthn':
      return 'Clé de sécurité (WebAuthn)';
    case 'recovery':
      return 'Codes de récupération';
    default:
      return factor.factor_type;
  }
}

function MfaSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-col gap-1">
        <Skeleton className="h-6 w-56" />
        <Skeleton className="h-4 w-72" />
      </div>
      {Array.from({ length: 2 }).map((_, i) => (
        <Card key={i}>
          <CardHeader>
            <div className="flex items-center gap-3">
              <Skeleton className="size-9 shrink-0 rounded-lg" />
              <div className="flex flex-col gap-1.5">
                <Skeleton className="h-4 w-36" />
                <Skeleton className="h-3 w-24" />
              </div>
            </div>
          </CardHeader>
        </Card>
      ))}
    </div>
  );
}

export default function MfaPage() {
  const navigate = useNavigate();
  const [factors, setFactors] = useState<MfaFactorView[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [removingId, setRemovingId] = useState<string | null>(null);
  const [showStepUp, setShowStepUp] = useState(false);
  const [stepUpMethod, setStepUpMethod] = useState<StepUpMethod | null>(null);
  const [stepUpPassword, setStepUpPassword] = useState('');
  const [stepUpTotpCode, setStepUpTotpCode] = useState('');
  const [stepUpRecoveryCode, setStepUpRecoveryCode] = useState('');
  const [stepUpLoading, setStepUpLoading] = useState(false);
  const [stepUpError, setStepUpError] = useState<string | null>(null);

  const fetchFactors = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await listMfaFactors('');
      setFactors(result.factors);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load MFA factors');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchFactors();
  }, [fetchFactors]);

  const hasTotp = factors.some((f) => f.factor_type === 'totp');
  const hasWebAuthn = factors.some((f) => f.factor_type === 'webauthn');
  const hasRecovery = factors.some((f) => f.factor_type === 'recovery');

  const handleRemove = async (factorId: string) => {
    setRemovingId(factorId);
    setStepUpError(null);
    try {
      await removeMfaFactor('', factorId);
      setFactors((prev) => prev.filter((f) => f.id !== factorId));
      setRemovingId(null);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to remove factor';
      if (message.includes('step_up_required')) {
        setShowStepUp(true);
      } else {
        setStepUpError(message);
        setRemovingId(null);
      }
    }
  };

  const handleStepUp = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setStepUpLoading(true);
    setStepUpError(null);

    try {
      if (stepUpMethod === 'totp') {
        await stepUp('', { totpCode: stepUpTotpCode });
      } else if (stepUpMethod === 'webauthn') {
        await completeWebAuthnStepUp('', undefined, {
          timeoutMs: WEBAUTHN_TIMEOUT_MS,
        });
      } else if (stepUpMethod === 'recovery') {
        await stepUp('', { recoveryCode: stepUpRecoveryCode });
      } else {
        await stepUp('', { password: stepUpPassword });
      }

      setShowStepUp(false);
      resetStepUpState();
      if (removingId) {
        await removeMfaFactor('', removingId);
        setFactors((prev) => prev.filter((f) => f.id !== removingId));
        setRemovingId(null);
      }
    } catch (err) {
      if (err instanceof MfaError) {
        setStepUpError(err.message);
      } else if (err instanceof WebauthnBrowserError && err.code === 'webauthn_timeout') {
        setStepUpError('La demande WebAuthn a expiré. Veuillez réessayer.');
      } else {
        setStepUpError(err instanceof Error ? err.message : 'Échec de la vérification');
      }
    } finally {
      setStepUpLoading(false);
    }
  };

  const resetStepUpState = () => {
    setStepUpMethod(null);
    setStepUpPassword('');
    setStepUpTotpCode('');
    setStepUpRecoveryCode('');
    setStepUpError(null);
  };

  const cancelStepUp = () => {
    setShowStepUp(false);
    resetStepUpState();
    setRemovingId(null);
  };

  if (loading) return <MfaSkeleton />;

  return (
    <div className="flex flex-col gap-6 animate-fade-slide-up [animation-delay:0ms]">
      <div>
        <Button
          variant="ghost"
          size="sm"
          className="mb-3 -ml-1 text-muted-foreground"
          onClick={() => void navigate({ to: '/account/security' })}
        >
          <ArrowLeft data-icon="inline-start" />
          Retour
        </Button>
        <h1 className="text-xl font-heading font-semibold">Authentification multi-facteurs</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Gérez les méthodes de vérification de votre compte.
        </p>
      </div>

      {error && (
        <div className="rounded-lg bg-destructive/10 px-3 py-2.5 text-sm text-destructive">
          {error}
        </div>
      )}

      {factors.length === 0 && !loading && (
        <div className="flex flex-col items-center gap-2 rounded-xl border border-dashed py-12 text-center">
          <ShieldAlert className="size-8 text-muted-foreground/50" />
          <p className="text-sm text-muted-foreground">Aucun facteur configuré.</p>
          <p className="text-xs text-muted-foreground/60">
            Ajoutez une méthode ci-dessous pour sécuriser votre compte.
          </p>
        </div>
      )}

      {factors.map((factor, index) => {
        const Icon = getFactorTypeIcon(factor.factor_type);
        const iconClass =
          factor.factor_type === 'recovery'
            ? 'size-4 text-muted-foreground'
            : 'size-4 text-primary';

        return (
          <Card
            key={factor.id}
            className="animate-fade-slide-up"
            style={{ animationDelay: `${(index + 1) * 100}ms` }}
          >
            <CardHeader>
              <div className="flex items-start gap-3">
                <div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-muted">
                  <Icon className={iconClass} />
                </div>
                <div className="flex min-w-0 flex-1 flex-col gap-1">
                  <div className="flex items-center gap-2">
                    <CardTitle className="truncate">
                      {factor.label ?? factorLabel(factor)}
                    </CardTitle>
                    <Badge
                      variant={factor.status === 'active' ? 'default' : 'secondary'}
                      className="shrink-0"
                    >
                      {factor.status === 'active' ? 'Actif' : 'En attente'}
                    </Badge>
                  </div>
                  {factor.last_used_at && (
                    <CardDescription>
                      Dernière utilisation : {new Date(factor.last_used_at).toLocaleDateString()}
                    </CardDescription>
                  )}
                </div>
              </div>
            </CardHeader>
            <CardFooter>
              <Button
                variant="outline"
                size="sm"
                disabled={removingId === factor.id}
                onClick={() => factor.id && handleRemove(factor.id)}
              >
                <Trash2 data-icon="inline-start" />
                {removingId === factor.id ? 'Suppression...' : 'Supprimer'}
              </Button>
            </CardFooter>
          </Card>
        );
      })}

      <Separator className="my-2" />

      <Card className="animate-fade-slide-up [animation-delay:300ms]">
        <CardHeader>
          <CardTitle>Ajouter une méthode</CardTitle>
          <CardDescription>
            Choisissez une méthode d'authentification supplémentaire.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-col gap-2">
          {FACTOR_ADD_ACTIONS.map((action) => {
            if (action.path === '/account/mfa/recovery-codes' && hasRecovery) {
              return null;
            }
            return (
              <Button
                key={action.path}
                variant="outline"
                className="justify-start gap-3"
                onClick={() => void navigate({ to: action.path })}
              >
                <action.icon className="size-4 text-muted-foreground" />
                {action.label}
              </Button>
            );
          })}
        </CardContent>
      </Card>

      <Dialog
        open={showStepUp}
        onOpenChange={(open) => {
          if (!open) cancelStepUp();
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Vérification requise</DialogTitle>
            <DialogDescription>
              Pour supprimer ce facteur, veuillez vérifier votre identité.
            </DialogDescription>
          </DialogHeader>

          {!stepUpMethod ? (
            <div className="flex flex-col gap-2">
              <Button
                variant="outline"
                className="justify-start gap-3"
                onClick={() => setStepUpMethod('password')}
              >
                <KeyRound className="size-4 text-muted-foreground" />
                Mot de passe
              </Button>
              {hasTotp && (
                <Button
                  variant="outline"
                  className="justify-start gap-3"
                  onClick={() => setStepUpMethod('totp')}
                >
                  <Smartphone className="size-4 text-muted-foreground" />
                  Code d'authentification (TOTP)
                </Button>
              )}
              {hasWebAuthn && (
                <Button
                  variant="outline"
                  className="justify-start gap-3"
                  onClick={() => setStepUpMethod('webauthn')}
                >
                  <Fingerprint className="size-4 text-muted-foreground" />
                  Passkey ou clé de sécurité
                </Button>
              )}
              {hasRecovery && (
                <Button
                  variant="outline"
                  className="justify-start gap-3"
                  onClick={() => setStepUpMethod('recovery')}
                >
                  <ShieldAlert className="size-4 text-muted-foreground" />
                  Code de récupération
                </Button>
              )}
            </div>
          ) : (
            <form onSubmit={handleStepUp} className="flex flex-col gap-4">
              {stepUpMethod === 'password' && (
                <Input
                  type="password"
                  placeholder="Mot de passe"
                  value={stepUpPassword}
                  onChange={(e) => setStepUpPassword(e.target.value)}
                  required
                  autoFocus
                />
              )}
              {stepUpMethod === 'totp' && (
                <Input
                  type="text"
                  inputMode="numeric"
                  autoComplete="one-time-code"
                  placeholder="000000"
                  maxLength={6}
                  value={stepUpTotpCode}
                  onChange={(e) => setStepUpTotpCode(e.target.value)}
                  required
                  autoFocus
                />
              )}
              {stepUpMethod === 'webauthn' && (
                <p className="text-sm text-muted-foreground">
                  Cliquez sur Confirmer pour utiliser votre passkey ou clé de sécurité.
                </p>
              )}
              {stepUpMethod === 'recovery' && (
                <Input
                  type="text"
                  placeholder="XXXX-XXXX-XXXX"
                  value={stepUpRecoveryCode}
                  onChange={(e) => setStepUpRecoveryCode(e.target.value)}
                  required
                  autoFocus
                />
              )}

              {stepUpError && <p className="text-sm text-destructive">{stepUpError}</p>}

              <DialogFooter>
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => {
                    setStepUpMethod(null);
                    setStepUpError(null);
                  }}
                  disabled={stepUpLoading}
                >
                  Retour
                </Button>
                <Button type="submit" disabled={stepUpLoading}>
                  {stepUpLoading ? 'Vérification...' : 'Confirmer'}
                </Button>
              </DialogFooter>
            </form>
          )}
        </DialogContent>
      </Dialog>
    </div>
  );
}
