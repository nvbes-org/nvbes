import { identityClient } from '@nvbes/identity-client';
import { fetchPowChallenge, solvePowChallenge } from '@nvbes/identity-sdk-web';
import { clientErrorMessage } from '@nvbes/web-runtime';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { zxcvbn, zxcvbnOptions } from '@zxcvbn-ts/core';
import { adjacencyGraphs, dictionary as commonDictionary } from '@zxcvbn-ts/language-common';
import { dictionary as frDictionary, translations } from '@zxcvbn-ts/language-fr';
import {
  ArrowLeftIcon,
  ArrowRightIcon,
  Building2Icon,
  CheckIcon,
  MapPinIcon,
  UserIcon,
} from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
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
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Spinner } from '@/components/ui/spinner';
import { cn } from '@/lib/utils';
import type { SupportedRegion } from '../identity.auth.api';
import {
  detectRegionQueryFn,
  identityAuthMutationKeys,
  submitRegisterMutationFn,
  supportedRegionsQueryFn,
} from '../identity.auth.queries';
import { identityApiBaseUrl } from '../identity.http';
import {
  authorizeIdentitySession,
  clearPendingOAuthAuthorizeRequest,
  readOAuthAuthorizeRequest,
  readPendingOAuthAuthorizeRequest,
  savePendingOAuthAuthorizeRequest,
} from '../identity.oauth';
import { trackEvent } from '../identity.posthog';
import { RegisterBrandPanel } from './RegisterBrandPanel';

type RegisterStep = 1 | 2;

const STEPS: { label: string; icon: typeof UserIcon }[] = [
  { label: 'Identité', icon: UserIcon },
  { label: 'Espace', icon: Building2Icon },
];

const options = {
  dictionary: {
    ...commonDictionary,
    ...frDictionary,
  },
  graphs: adjacencyGraphs,
  translations,
};
zxcvbnOptions.setOptions(options);

function countryCodeToFlag(code: string): string {
  const chars = code.toUpperCase().split('');
  if (chars.length !== 2) return '';
  return String.fromCodePoint(
    chars[0].charCodeAt(0) - 65 + 0x1f1e6,
    chars[1].charCodeAt(0) - 65 + 0x1f1e6,
  );
}

function ErrorMessage({ message }: { message: string }) {
  return (
    <div className="rounded-lg border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm text-destructive">
      {message}
    </div>
  );
}

function RegisterProgress({ step }: { step: RegisterStep }) {
  return (
    <div className="mb-8">
      <div className="flex items-center gap-2">
        {STEPS.map((s, i) => {
          const stepNum = i + 1;
          const active = step === stepNum;
          const complete = step > stepNum;
          const Icon = s.icon;
          return (
            <div key={s.label} className="flex items-center gap-2">
              <div
                className={cn(
                  'flex items-center gap-2 rounded-full px-3 py-1.5 text-xs font-medium transition-all duration-500',
                  active && 'bg-primary text-primary-foreground shadow-sm',
                  complete && 'bg-primary/10 text-primary',
                  !active && !complete && 'bg-muted text-muted-foreground',
                )}
              >
                <Icon className="size-3.5" />
                <span>
                  {stepNum}. {s.label}
                </span>
              </div>
              {i < STEPS.length - 1 && (
                <div
                  className={cn(
                    'h-px w-8 transition-colors duration-500',
                    complete ? 'bg-primary/50' : 'bg-border',
                  )}
                />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}

function PasswordStrengthMeter({ password }: { password: string }) {
  const result = useMemo(() => {
    if (!password) return null;
    return zxcvbn(password);
  }, [password]);

  if (!result) return null;

  const score = result.score;
  const bars = [
    { score: 1, label: 'très faible' },
    { score: 2, label: 'faible' },
    { score: 3, label: 'bon' },
    { score: 4, label: 'fort' },
  ];

  const getBarColor = (barScore: number) => {
    if (score >= barScore) {
      if (score <= 1) return 'bg-destructive';
      if (score === 2) return 'bg-orange-500';
      if (score === 3) return 'bg-blue-500';
      return 'bg-emerald-500';
    }
    return 'bg-muted';
  };

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex h-1.5 gap-1">
        {bars.map((bar) => (
          <div
            key={bar.score}
            className={cn(
              'flex-1 rounded-full transition-colors duration-300',
              getBarColor(bar.score),
            )}
          />
        ))}
      </div>
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground">
          {score <= 1 ? 'Très faible' : score === 2 ? 'Faible' : score === 3 ? 'Bon' : 'Fort'}
        </span>
        {result.feedback.warning && (
          <span className="max-w-[200px] truncate text-xs text-destructive">
            {result.feedback.warning}
          </span>
        )}
      </div>
    </div>
  );
}

function RegionSelect({
  detectedRegion,
  reliability,
  loading,
  regions,
  value,
  onValueChange,
}: {
  detectedRegion: string | null;
  reliability: 'high' | 'medium' | 'low' | 'none';
  loading: boolean;
  regions: SupportedRegion[];
  value: string;
  onValueChange: (value: string) => void;
}) {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-2">
        <Label htmlFor="register-region">Région</Label>
        {loading && (
          <span className="inline-flex items-center gap-1 text-xs text-muted-foreground">
            <Spinner className="size-3" />
            Détection en cours...
          </span>
        )}
        {!loading && detectedRegion && (
          <span
            className={cn(
              'inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-xs font-medium transition-colors',
              reliability === 'high' && 'border-emerald-500/30 bg-emerald-500/5 text-emerald-600',
              reliability === 'medium' && 'border-amber-500/30 bg-amber-500/5 text-amber-600',
              reliability === 'low' && 'border-blue-500/30 bg-blue-500/5 text-blue-600',
              reliability === 'none' && 'border-border bg-muted text-muted-foreground',
            )}
          >
            <MapPinIcon className="size-3" />
            Détecté (
            {reliability === 'high' ? 'IP' : reliability === 'medium' ? 'Timezone' : 'Locale'}) :{' '}
            {detectedRegion}
          </span>
        )}
      </div>
      <Select value={value} onValueChange={onValueChange}>
        <SelectTrigger id="register-region" className="w-full">
          <SelectValue placeholder="Sélectionnez votre pays..." />
        </SelectTrigger>
        <SelectContent>
          <SelectGroup>
            {regions.map((entry) => (
              <SelectItem key={entry.country_code} value={entry.country_code}>
                <span className="flex items-center gap-2">
                  <span className="text-base leading-none">
                    {countryCodeToFlag(entry.country_code)}
                  </span>
                  <span>{entry.display_name ?? entry.country_code}</span>
                  {entry.sub_region && (
                    <span className="text-xs text-muted-foreground">({entry.sub_region})</span>
                  )}
                </span>
              </SelectItem>
            ))}
          </SelectGroup>
        </SelectContent>
      </Select>
      <p className="text-xs text-muted-foreground">
        {detectedRegion
          ? 'Région détectée automatiquement. Vous pouvez la modifier si nécessaire.'
          : 'Sélectionnez le pays où vos données seront stockées.'}
      </p>
    </div>
  );
}

export default function RegisterPage() {
  const navigate = useNavigate();
  const location = useLocation();
  const searchParams = new URLSearchParams(location.searchStr);
  const oauthRequest = readOAuthAuthorizeRequest(searchParams);

  const [step, setStep] = useState<RegisterStep>(1);
  const [firstname, setFirstname] = useState('');
  const [lastname, setLastname] = useState('');
  const [username, setUsername] = useState('');
  const [birthdate, setBirthdate] = useState('');
  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [workspaceName, setWorkspaceName] = useState('');
  const [selectedRegion, setSelectedRegion] = useState('');
  const [checkingAuth, setCheckingAuth] = useState(true);

  useEffect(() => {
    const check = async () => {
      try {
        await identityClient.getMe();
        const pendingOauth =
          readOAuthAuthorizeRequest(new URLSearchParams(window.location.search)) ??
          readPendingOAuthAuthorizeRequest();
        if (pendingOauth) {
          await authorizeIdentitySession(null, pendingOauth);
          clearPendingOAuthAuthorizeRequest();
          return;
        }
        void navigate({ to: '/account' });
      } catch {
        setCheckingAuth(false);
      }
    };
    void check();
  }, [navigate]);

  const regionQuery = useQuery({
    queryKey: identityAuthMutationKeys.regionDetect,
    queryFn: detectRegionQueryFn,
    staleTime: 0,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: true,
  });

  const supportedRegionsQuery = useQuery({
    queryKey: identityAuthMutationKeys.supportedRegions,
    queryFn: supportedRegionsQueryFn,
    staleTime: 30 * 60 * 1000,
    gcTime: 30 * 60 * 1000,
    refetchOnWindowFocus: false,
  });

  const registerMutation = useMutation({
    mutationKey: identityAuthMutationKeys.register,
    mutationFn: submitRegisterMutationFn,
  });

  const detectedRegion = regionQuery.data?.region ?? null;
  const detectedReliability = regionQuery.data?.reliability ?? 'none';
  const supportedRegions = supportedRegionsQuery.data ?? [];
  const loading = registerMutation.isPending;
  const error = registerMutation.error
    ? clientErrorMessage(registerMutation.error, "Échec de l'inscription.")
    : null;

  useEffect(() => {
    if (detectedRegion && !selectedRegion) {
      setSelectedRegion(detectedRegion);
    }
  }, [detectedRegion, selectedRegion]);

  const today = new Date();
  const minBirthdate = new Date(today.getFullYear() - 120, today.getMonth(), today.getDate())
    .toISOString()
    .split('T')[0];
  const maxBirthdate = new Date(today.getFullYear() - 13, today.getMonth(), today.getDate())
    .toISOString()
    .split('T')[0];

  const passwordResult = useMemo(() => {
    if (!password) return null;
    return zxcvbn(password);
  }, [password]);

  const passwordScore = passwordResult?.score ?? 0;
  const canProceedFromStep1 = passwordScore >= 3;

  const handleStep1Next = () => {
    if (!canProceedFromStep1) return;
    setStep(2);
  };

  const handleStep2Back = () => {
    setStep(1);
  };

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    if (!canProceedFromStep1) return;

    try {
      trackEvent('auth.signup_started', {
        country: selectedRegion || detectedRegion || undefined,
      });

      const challenge = await fetchPowChallenge(identityApiBaseUrl);
      const powResolved =
        challenge && challenge.difficulty > 0
          ? {
              powNonce: challenge.nonce,
              powSolution: String(await solvePowChallenge(challenge.nonce, challenge.difficulty)),
            }
          : {};

      const result = await registerMutation.mutateAsync({
        data: {
          email,
          firstname: firstname || undefined,
          lastname: lastname || undefined,
          username,
          birthdate: birthdate || undefined,
          password,
          workspace_name: workspaceName,
          region: selectedRegion || undefined,
        },
        ...powResolved,
      });

      trackEvent('auth.signup_completed', {
        country: selectedRegion || detectedRegion || undefined,
      });

      if (oauthRequest) {
        savePendingOAuthAuthorizeRequest(oauthRequest);
      }

      void navigate({
        to: '/verify',
        state: (s) => ({
          ...s,
          email,
          resendAvailableAt: result.verification_resend_available_at,
        }),
      });
    } catch {
      // error handled via registerMutation.error
    }
  };

  if (checkingAuth) return null;

  return (
    <div className="flex min-h-screen">
      <RegisterBrandPanel />

      {/* Form Panel */}
      <div className="flex flex-1 items-center justify-center bg-muted/30 p-4 sm:p-8">
        <div className="w-full max-w-sm animate-fade-slide-up [animation-delay:150ms]">
          {/* Mobile-only brand header */}
          <div className="mb-8 flex items-center gap-2.5 lg:hidden">
            <div className="inline-flex size-8 items-center justify-center rounded-lg bg-primary/15">
              <div className="size-3 rounded-sm bg-primary" />
            </div>
            <span className="text-lg font-semibold tracking-tight text-foreground/85">nvbes</span>
          </div>

          <RegisterProgress step={step} />

          <Card>
            <CardHeader>
              <CardTitle>{step === 1 ? 'Créer un compte' : 'Espace de travail'}</CardTitle>
              <CardDescription>
                {step === 1
                  ? 'Renseignez vos informations personnelles.'
                  : 'Configurez votre espace de travail.'}
              </CardDescription>
            </CardHeader>

            <CardContent>
              {step === 1 && (
                <form
                  onSubmit={(e) => {
                    e.preventDefault();
                    handleStep1Next();
                  }}
                  className="flex flex-col gap-5"
                >
                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-2">
                      <Label htmlFor="register-firstname">Prénom</Label>
                      <Input
                        id="register-firstname"
                        type="text"
                        placeholder="Prénom"
                        value={firstname}
                        onChange={(e) => setFirstname(e.target.value)}
                      />
                    </div>
                    <div className="flex flex-col gap-2">
                      <Label htmlFor="register-lastname">Nom</Label>
                      <Input
                        id="register-lastname"
                        type="text"
                        placeholder="Nom"
                        value={lastname}
                        onChange={(e) => setLastname(e.target.value)}
                      />
                    </div>
                  </div>

                  <div className="flex flex-col gap-2">
                    <Label htmlFor="register-email">Email</Label>
                    <Input
                      id="register-email"
                      type="email"
                      placeholder="vous@exemple.fr"
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                      required
                      autoComplete="email"
                      autoFocus
                    />
                  </div>

                  <div className="grid grid-cols-2 gap-3">
                    <div className="flex flex-col gap-2">
                      <Label htmlFor="register-username">Nom d&apos;utilisateur</Label>
                      <Input
                        id="register-username"
                        type="text"
                        placeholder="Nom d'utilisateur"
                        value={username}
                        onChange={(e) => setUsername(e.target.value)}
                        required
                        autoComplete="username"
                      />
                    </div>
                    <div className="flex flex-col gap-2">
                      <Label htmlFor="register-birthdate">Date de naissance</Label>
                      <Input
                        id="register-birthdate"
                        type="date"
                        min={minBirthdate}
                        max={maxBirthdate}
                        value={birthdate}
                        onChange={(e) => setBirthdate(e.target.value)}
                        required
                      />
                    </div>
                  </div>

                  <div className="flex flex-col gap-2">
                    <Label htmlFor="register-password">Mot de passe</Label>
                    <Input
                      id="register-password"
                      type="password"
                      placeholder="••••••••"
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      required
                      autoComplete="new-password"
                    />
                    <PasswordStrengthMeter password={password} />
                  </div>

                  <Button
                    type="submit"
                    disabled={!canProceedFromStep1}
                    className="w-full"
                    size="lg"
                  >
                    Continuer
                    <ArrowRightIcon data-icon="inline-end" />
                  </Button>
                </form>
              )}

              {step === 2 && (
                <form onSubmit={handleSubmit} className="flex flex-col gap-5">
                  <div className="flex flex-col gap-2">
                    <Label htmlFor="register-workspace">Nom du workspace</Label>
                    <Input
                      id="register-workspace"
                      type="text"
                      placeholder="Mon workspace"
                      value={workspaceName}
                      onChange={(e) => setWorkspaceName(e.target.value)}
                      required
                      autoFocus
                    />
                  </div>

                  <RegionSelect
                    detectedRegion={detectedRegion}
                    reliability={detectedReliability}
                    loading={regionQuery.isPending}
                    regions={supportedRegions}
                    value={selectedRegion}
                    onValueChange={setSelectedRegion}
                  />

                  {error && <ErrorMessage message={error} />}

                  <div className="flex gap-2">
                    <Button
                      type="button"
                      variant="outline"
                      className="flex-1"
                      onClick={handleStep2Back}
                      disabled={loading}
                    >
                      <ArrowLeftIcon data-icon="inline-start" />
                      Retour
                    </Button>
                    <Button type="submit" className="flex-1" disabled={loading}>
                      {loading ? (
                        <>
                          <Spinner data-icon="inline-start" />
                          Inscription...
                        </>
                      ) : (
                        <>
                          S&apos;inscrire
                          <CheckIcon data-icon="inline-end" />
                        </>
                      )}
                    </Button>
                  </div>
                </form>
              )}
            </CardContent>

            <CardFooter className="justify-center">
              <p className="text-center text-sm text-muted-foreground">
                Déjà un compte ?{' '}
                <a
                  href={`/login${location.searchStr}`}
                  className="font-medium text-primary hover:underline"
                >
                  Se connecter
                </a>
              </p>
            </CardFooter>
          </Card>
        </div>
      </div>
    </div>
  );
}
