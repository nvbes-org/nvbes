import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState, type FormEvent } from 'react';
import {
  changeAccountPassword,
  listAccountMfaFactors,
  removeAccountMfaFactor,
  verifyAccountStepUp,
} from '@/account.identity';
import { AccountPage } from '@/components/AccountPage';
import { AccountPageError, AccountPageLoading } from '@/components/AccountPageState';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { MfaEnrollmentSection } from '@/pages/AccountSecurityPage.enrollment';
import { MutationStatus, SecuritySection } from '@/pages/AccountSecurityPage.shared';

const factorsKey = ['account', 'identity', 'mfa-factors'] as const;
type MfaFactor = Awaited<ReturnType<typeof listAccountMfaFactors>>['factors'][number];

export default function AccountSecurityPage() {
  const queryClient = useQueryClient();
  const factors = useQuery({
    queryKey: factorsKey,
    queryFn: () => listAccountMfaFactors({ limit: 100 }),
  });
  const refresh = () => queryClient.invalidateQueries({ queryKey: factorsKey });

  return (
    <AccountPage
      title="Sécurité"
      description="Gérez les moyens d’authentification de votre compte. Les secrets restent protégés par Identity."
    >
      <StepUpSection />
      <PasswordSection />
      <MfaEnrollmentSection onChanged={refresh} />
      {factors.isPending ? (
        <AccountPageLoading label="Chargement des facteurs…" />
      ) : factors.error || !factors.data ? (
        <AccountPageError error={factors.error} onRetry={() => void factors.refetch()} />
      ) : (
        <FactorList factors={factors.data.factors} onChanged={refresh} />
      )}
    </AccountPage>
  );
}

function StepUpSection() {
  const [method, setMethod] = useState<'password' | 'totp' | 'recovery'>('password');
  const [value, setValue] = useState('');
  const mutation = useMutation({
    mutationFn: () =>
      verifyAccountStepUp(
        method === 'password'
          ? { password: value }
          : method === 'totp'
            ? { totpCode: value }
            : { recoveryCode: value },
      ),
    onSuccess: () => setValue(''),
  });
  return (
    <SecuritySection
      title="Vérification renforcée"
      description="Confirmez votre identité avant une opération sensible."
    >
      <div className="flex flex-wrap gap-2">
        {(['password', 'totp', 'recovery'] as const).map((candidate) => (
          <Button
            key={candidate}
            type="button"
            variant={method === candidate ? 'secondary' : 'outline'}
            onClick={() => setMethod(candidate)}
          >
            {candidate === 'password'
              ? 'Mot de passe'
              : candidate === 'totp'
                ? 'Code TOTP'
                : 'Code de récupération'}
          </Button>
        ))}
      </div>
      <div className="flex max-w-lg gap-2">
        <Input
          type={method === 'password' ? 'password' : 'text'}
          value={value}
          onChange={(event) => setValue(event.target.value)}
          autoComplete={method === 'password' ? 'current-password' : 'one-time-code'}
        />
        <Button
          type="button"
          disabled={!value || mutation.isPending}
          onClick={() => mutation.mutate()}
        >
          Vérifier
        </Button>
      </div>
      <MutationStatus
        mutation={mutation}
        success="Identité confirmée pour les prochaines opérations sensibles."
      />
    </SecuritySection>
  );
}

function PasswordSection() {
  const [password, setPassword] = useState('');
  const [confirmation, setConfirmation] = useState('');
  const mutation = useMutation({ mutationFn: () => changeAccountPassword(password) });
  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (password.length >= 12 && password === confirmation) mutation.mutate();
  };
  return (
    <SecuritySection
      title="Mot de passe"
      description="Utilisez au minimum 12 caractères et un mot de passe unique."
    >
      <form className="grid max-w-lg gap-3" onSubmit={submit}>
        <Label htmlFor="new-password">Nouveau mot de passe</Label>
        <Input
          id="new-password"
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete="new-password"
        />
        <Label htmlFor="confirm-password">Confirmation</Label>
        <Input
          id="confirm-password"
          type="password"
          value={confirmation}
          onChange={(e) => setConfirmation(e.target.value)}
          autoComplete="new-password"
        />
        {confirmation && password !== confirmation ? (
          <p className="text-sm text-destructive">Les mots de passe ne correspondent pas.</p>
        ) : null}
        <Button
          type="submit"
          className="w-fit"
          disabled={password.length < 12 || password !== confirmation || mutation.isPending}
        >
          Modifier le mot de passe
        </Button>
      </form>
      <MutationStatus mutation={mutation} success="Mot de passe modifié." />
    </SecuritySection>
  );
}

function FactorList({
  factors,
  onChanged,
}: {
  factors: MfaFactor[];
  onChanged: () => Promise<unknown>;
}) {
  const remove = useMutation({ mutationFn: removeAccountMfaFactor, onSuccess: onChanged });
  return (
    <SecuritySection
      title="Facteurs enregistrés"
      description={`${factors.length} facteur(s) actif(s).`}
    >
      {factors.length === 0 ? (
        <p className="text-sm text-muted-foreground">Aucun facteur MFA enregistré.</p>
      ) : (
        <div className="divide-y divide-border border-y border-border">
          {factors.map((factor) => (
            <div key={factor.id} className="flex items-center justify-between gap-4 py-4">
              <div>
                <p className="text-sm font-medium">
                  {factor.label || factor.kind || factor.factor_type}
                </p>
                <p className="text-xs text-muted-foreground">
                  {factor.factor_type} · {factor.status}
                </p>
              </div>
              <Button
                type="button"
                variant="destructive"
                disabled={remove.isPending}
                onClick={() => remove.mutate(factor.id)}
              >
                Supprimer
              </Button>
            </div>
          ))}
        </div>
      )}
      <MutationStatus mutation={remove} />
    </SecuritySection>
  );
}
