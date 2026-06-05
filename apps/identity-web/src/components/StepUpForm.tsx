import { useEffect, useState } from 'react';
import { listMfaFactors, stepUp, completeWebAuthnStepUp } from '@nvbes/identity-sdk-web';
import type { MfaFactorView } from '@nvbes/identity-sdk-core/src/types';
import { Button } from '@/components/ui/button';
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
import { identityHttpClient } from '../identity.http';
import { z } from 'zod';
import { ShieldCheck, ShieldAlert, KeyRound, Smartphone, LifeBuoy, Key } from 'lucide-react';

const PreferencesSchema = z.object({
  theme: z.string(),
  language: z.string(),
  skip_password: z.boolean().default(false),
});

export type StepUpMethod = 'password' | 'webauthn' | 'totp' | 'recovery';

interface StepUpFormProps {
  onSuccess: () => void;
  onCancel: () => void;
  description?: string;
}

export default function StepUpForm({ onSuccess, onCancel, description }: StepUpFormProps) {
  const [method, setMethod] = useState<StepUpMethod>('password');
  const [factors, setFactors] = useState<MfaFactorView[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Form fields
  const [password, setPassword] = useState('');
  const [totpCode, setTotpCode] = useState('');
  const [recoveryCode, setRecoveryCode] = useState('');

  const hasTotp = factors.some((f) => f.factor_type === 'totp');
  const hasWebAuthn = factors.some((f) => f.factor_type === 'webauthn');
  const hasRecovery = factors.some((f) => f.factor_type === 'recovery');

  useEffect(() => {
    let active = true;
    async function load() {
      try {
        const [factorsRes, prefsRes] = await Promise.all([
          listMfaFactors(''),
          identityHttpClient.request('/auth/me/preferences', PreferencesSchema),
        ]);
        if (!active) return;
        setFactors(factorsRes.factors);

        const canSkip =
          (prefsRes.skip_password ?? false) &&
          factorsRes.factors.some((f) => f.factor_type === 'webauthn');
        if (canSkip) {
          setMethod('webauthn');
        }
      } catch (err) {
        console.error('Failed to load step-up pre-requisites:', err);
      }
    }
    void load();
    return () => {
      active = false;
    };
  }, []);

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    try {
      if (method === 'password') {
        await stepUp('', { password });
      } else if (method === 'totp') {
        await stepUp('', { totpCode });
      } else if (method === 'recovery') {
        await stepUp('', { recoveryCode });
      } else if (method === 'webauthn') {
        await completeWebAuthnStepUp('');
      }
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Vérification échouée');
    } finally {
      setLoading(false);
    }
  };

  const handleWebAuthnClick = async () => {
    setLoading(true);
    setError(null);
    try {
      await completeWebAuthnStepUp('');
      onSuccess();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Authentification WebAuthn échouée');
    } finally {
      setLoading(false);
    }
  };

  return (
    <form
      onSubmit={handleSubmit}
      className="w-full max-w-md space-y-4 rounded-xl border bg-card p-6 shadow-sm"
    >
      <div className="flex flex-col gap-1.5 text-center">
        <div className="mx-auto flex size-12 items-center justify-center rounded-full bg-primary/10 text-primary">
          <ShieldCheck className="size-6" />
        </div>
        <h1 className="text-xl font-heading font-semibold mt-2">Vérification requise</h1>
        <p className="text-xs text-muted-foreground">
          {description || 'Veuillez confirmer votre identité pour continuer.'}
        </p>
      </div>

      {/* Select authentication method */}
      <div className="space-y-2">
        <Label htmlFor="stepup-method" className="text-xs">
          Méthode de vérification
        </Label>
        <Select value={method} onValueChange={(val) => setMethod(val as StepUpMethod)}>
          <SelectTrigger id="stepup-method" className="w-full text-xs">
            <SelectValue placeholder="Choisir une méthode" />
          </SelectTrigger>
          <SelectContent>
            <SelectGroup>
              <SelectItem value="password" className="text-xs">
                <span className="flex items-center gap-2">
                  <Key className="size-3.5" /> Mot de passe
                </span>
              </SelectItem>
              {hasWebAuthn && (
                <SelectItem value="webauthn" className="text-xs">
                  <span className="flex items-center gap-2">
                    <KeyRound className="size-3.5" /> Passkey / Clé de sécurité
                  </span>
                </SelectItem>
              )}
              {hasTotp && (
                <SelectItem value="totp" className="text-xs">
                  <span className="flex items-center gap-2">
                    <Smartphone className="size-3.5" /> Code TOTP
                  </span>
                </SelectItem>
              )}
              {hasRecovery && (
                <SelectItem value="recovery" className="text-xs">
                  <span className="flex items-center gap-2">
                    <LifeBuoy className="size-3.5" /> Code de récupération
                  </span>
                </SelectItem>
              )}
            </SelectGroup>
          </SelectContent>
        </Select>
      </div>

      {/* Dynamic input field depending on method */}
      {method === 'password' && (
        <div className="space-y-2">
          <Label htmlFor="stepup-password" className="text-xs">
            Mot de passe
          </Label>
          <Input
            id="stepup-password"
            type="password"
            placeholder="Saisissez votre mot de passe"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            className="text-xs"
            required
            autoFocus
          />
        </div>
      )}

      {method === 'totp' && (
        <div className="space-y-2">
          <Label htmlFor="stepup-totp" className="text-xs">
            Code d'authentification
          </Label>
          <Input
            id="stepup-totp"
            type="text"
            inputMode="numeric"
            placeholder="000000"
            maxLength={6}
            value={totpCode}
            onChange={(e) => setTotpCode(e.target.value)}
            className="text-xs text-center tracking-widest font-mono"
            required
            autoFocus
          />
        </div>
      )}

      {method === 'recovery' && (
        <div className="space-y-2">
          <Label htmlFor="stepup-recovery" className="text-xs">
            Code de récupération
          </Label>
          <Input
            id="stepup-recovery"
            type="text"
            placeholder="XXXX-XXXX-XXXX"
            value={recoveryCode}
            onChange={(e) => setRecoveryCode(e.target.value)}
            className="text-xs font-mono"
            required
            autoFocus
          />
        </div>
      )}

      {method === 'webauthn' && (
        <div className="py-3 flex flex-col items-center justify-center gap-3">
          <p className="text-xs text-muted-foreground text-center">
            Utilisez votre authentification biométrique ou clé de sécurité physique.
          </p>
          <Button
            type="button"
            onClick={handleWebAuthnClick}
            disabled={loading}
            className="w-full text-xs gap-2"
          >
            <KeyRound className="size-4" />
            {loading ? 'Attente de la clé...' : 'Déclencher la clé de sécurité'}
          </Button>
        </div>
      )}

      {error && (
        <div className="flex items-center gap-2 rounded-lg bg-destructive/10 p-3 text-xs text-destructive">
          <ShieldAlert className="size-4 shrink-0" />
          <span>{error}</span>
        </div>
      )}

      <div className="flex gap-2 pt-2">
        <Button
          type="button"
          variant="outline"
          className="flex-1 text-xs"
          onClick={onCancel}
          disabled={loading}
        >
          Annuler
        </Button>
        {method !== 'webauthn' && (
          <Button type="submit" className="flex-1 text-xs" disabled={loading}>
            {loading ? 'Vérification...' : 'Confirmer'}
          </Button>
        )}
      </div>
    </form>
  );
}
