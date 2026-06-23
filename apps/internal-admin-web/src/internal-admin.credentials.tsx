import { KeyRound, RotateCcw, ShieldCheck } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import type { AdminCredentials } from './internal-admin.types';

type CredentialsPanelProps = {
  credentials: AdminCredentials;
  isReady: boolean;
  onChange: (credentials: AdminCredentials) => void;
  onReset: () => void;
};

export function CredentialsPanel({
  credentials,
  isReady,
  onChange,
  onReset,
}: CredentialsPanelProps) {
  return (
    <section
      className="border-border bg-card text-card-foreground rounded-lg border p-4"
      id="contexte"
    >
      <div className="mb-4 flex items-start justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="bg-primary/10 text-primary flex size-9 items-center justify-center rounded-md">
            <ShieldCheck className="size-4" />
          </div>
          <div>
            <h2 className="text-sm font-semibold">Contexte operateur</h2>
            <p className="text-muted-foreground text-xs">
              Headers internes envoyes vers internal-admin-api.
            </p>
          </div>
        </div>
        <Badge variant={isReady ? 'default' : 'secondary'}>{isReady ? 'Pret' : 'Incomplet'}</Badge>
      </div>

      <div className="grid gap-3">
        <Field label="Workspace ID">
          <Input
            className="font-mono text-xs"
            value={credentials.workspaceId}
            onChange={(event) => onChange({ ...credentials, workspaceId: event.target.value })}
            placeholder="00000000-0000-0000-0000-000000000000"
          />
        </Field>
        <Field label="Actor principal ID">
          <Input
            className="font-mono text-xs"
            value={credentials.actorPrincipalId}
            onChange={(event) => onChange({ ...credentials, actorPrincipalId: event.target.value })}
            placeholder="00000000-0000-0000-0000-000000000000"
          />
        </Field>
        <Field label="Internal token">
          <div className="relative">
            <KeyRound className="text-muted-foreground pointer-events-none absolute top-2.5 left-3 size-4" />
            <Input
              className="pl-9 font-mono text-xs"
              value={credentials.internalToken}
              onChange={(event) => onChange({ ...credentials, internalToken: event.target.value })}
              placeholder="x-nvbes-internal-token"
              type="password"
            />
          </div>
        </Field>
      </div>
      <Button className="mt-3 w-full" onClick={onReset} type="button" variant="outline">
        <RotateCcw className="size-4" />
        Effacer le contexte local
      </Button>
    </section>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <div className="grid gap-1.5">
      <Label className="text-xs">{label}</Label>
      {children}
    </div>
  );
}
