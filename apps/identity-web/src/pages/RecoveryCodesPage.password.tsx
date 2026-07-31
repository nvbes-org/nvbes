import type { SubmitEvent } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';

export function RecoveryCodesPasswordStep({
  error,
  loading,
  password,
  onCancel,
  onPasswordChange,
  onSubmit,
}: {
  error: string | null;
  loading: boolean;
  password: string;
  onCancel: () => void;
  onPasswordChange: (value: string) => void;
  onSubmit: (event: SubmitEvent<HTMLFormElement>) => void;
}) {
  return (
    <div className="mx-auto w-full max-w-md space-y-4">
      <form onSubmit={onSubmit} className="space-y-4">
        <h1 className="text-2xl font-bold">Codes de récupération</h1>
        <p className="text-sm text-muted-foreground">
          Confirmez votre mot de passe pour générer de nouveaux codes de récupération.
        </p>
        <Input
          type="password"
          placeholder="Mot de passe"
          value={password}
          onChange={(event) => onPasswordChange(event.target.value)}
          required
        />
        {error ? <p className="text-sm text-destructive">{error}</p> : null}
        <div className="flex gap-2">
          <Button type="button" variant="outline" className="flex-1" onClick={onCancel}>
            Annuler
          </Button>
          <Button type="submit" className="flex-1" disabled={loading}>
            {loading ? 'Génération...' : 'Générer'}
          </Button>
        </div>
      </form>
    </div>
  );
}
