import { useMutation } from '@tanstack/react-query';
import { useState } from 'react';
import {
  beginAccountTotp,
  confirmAccountTotp,
  generateAccountRecoveryCodes,
  registerAccountWebAuthn,
} from '@/account.identity';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { MutationStatus, SecuritySection } from '@/pages/AccountSecurityPage.shared';

export function MfaEnrollmentSection({ onChanged }: { onChanged: () => Promise<unknown> }) {
  const [label, setLabel] = useState('');
  const [totp, setTotp] = useState<{ factorId: string; secret: string; uri: string } | null>(null);
  const [code, setCode] = useState('');
  const [recoveryCodes, setRecoveryCodes] = useState<string[]>([]);
  const totpSetup = useMutation({
    mutationFn: () => beginAccountTotp(label || undefined),
    onSuccess: (result) =>
      setTotp({
        factorId: result.factor.id,
        secret: result.secret_base32,
        uri: result.provisioning_uri,
      }),
  });
  const totpConfirm = useMutation({
    mutationFn: () => confirmAccountTotp(totp?.factorId ?? '', code),
    onSuccess: async () => {
      setTotp(null);
      setCode('');
      await onChanged();
    },
  });
  const webauthn = useMutation({
    mutationFn: (kind: 'passkey' | 'security_key') =>
      registerAccountWebAuthn(kind, label || undefined),
    onSuccess: onChanged,
  });
  const recovery = useMutation({
    mutationFn: generateAccountRecoveryCodes,
    onSuccess: (result) => setRecoveryCodes(result.codes),
  });

  return (
    <SecuritySection
      title="Ajouter un facteur"
      description="Enregistrez une passkey, une clé physique ou une application TOTP."
    >
      <div className="max-w-lg space-y-2">
        <Label htmlFor="factor-label">Nom du facteur</Label>
        <Input id="factor-label" value={label} onChange={(event) => setLabel(event.target.value)} />
      </div>
      <div className="flex flex-wrap gap-2">
        <Button
          type="button"
          onClick={() => webauthn.mutate('passkey')}
          disabled={webauthn.isPending}
        >
          Ajouter une passkey
        </Button>
        <Button
          type="button"
          variant="outline"
          onClick={() => webauthn.mutate('security_key')}
          disabled={webauthn.isPending}
        >
          Ajouter une clé de sécurité
        </Button>
        <Button
          type="button"
          variant="outline"
          onClick={() => totpSetup.mutate()}
          disabled={totpSetup.isPending}
        >
          Configurer TOTP
        </Button>
        <Button
          type="button"
          variant="outline"
          onClick={() => recovery.mutate()}
          disabled={recovery.isPending}
        >
          Régénérer les codes de récupération
        </Button>
      </div>
      {totp ? (
        <div className="max-w-lg space-y-3 border-l-2 border-primary pl-4">
          <p className="break-all text-sm">
            Secret : <code>{totp.secret}</code>
          </p>
          <a className="break-all text-sm text-primary underline" href={totp.uri}>
            Ouvrir dans l’application d’authentification
          </a>
          <Input
            value={code}
            onChange={(event) => setCode(event.target.value)}
            inputMode="numeric"
            autoComplete="one-time-code"
            placeholder="Code à 6 chiffres"
          />
          <Button
            type="button"
            disabled={!code || totpConfirm.isPending}
            onClick={() => totpConfirm.mutate()}
          >
            Confirmer TOTP
          </Button>
        </div>
      ) : null}
      {recoveryCodes.length ? (
        <Alert>
          <AlertDescription>
            <p className="font-medium text-foreground">
              Conservez ces codes hors ligne. Ils ne seront plus affichés.
            </p>
            <ul className="mt-2 grid grid-cols-2 gap-1 font-mono">
              {recoveryCodes.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          </AlertDescription>
        </Alert>
      ) : null}
      <MutationStatus mutation={totpSetup} />
      <MutationStatus mutation={totpConfirm} />
      <MutationStatus mutation={webauthn} />
      <MutationStatus mutation={recovery} />
    </SecuritySection>
  );
}
