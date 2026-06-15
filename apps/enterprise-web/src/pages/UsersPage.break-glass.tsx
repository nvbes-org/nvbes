import { Ban, KeyRound, ShieldAlert } from 'lucide-react';
import { useState } from 'react';
import { Alert, AlertDescription, AlertTitle } from '../components/ui/alert';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { Field, FieldError, FieldLabel } from '../components/ui/field';
import { Input } from '../components/ui/input';
import { Textarea } from '../components/ui/textarea';
import type { EnterpriseUserRow } from './UsersPage.table';

export type BreakGlassFormValue = {
  reason: string;
  procedure_reference: string;
};

type BreakGlassPanelProps = {
  user: EnterpriseUserRow;
  canManageBreakGlass: boolean;
  pending: boolean;
  error: Error | null;
  onActivate: (input: BreakGlassFormValue) => Promise<boolean>;
  onRevoke: (reason: string) => Promise<boolean>;
};

export function BreakGlassPanel({
  user,
  canManageBreakGlass,
  pending,
  error,
  onActivate,
  onRevoke,
}: BreakGlassPanelProps) {
  const [reason, setReason] = useState('');
  const [procedureReference, setProcedureReference] = useState('');
  const [fieldError, setFieldError] = useState<string | null>(null);
  const active = Boolean(user.break_glass);

  async function submit() {
    if (reason.trim().length === 0) {
      setFieldError('An audit reason is required.');
      return;
    }
    if (!active && procedureReference.trim().length === 0) {
      setFieldError('A procedure reference is required.');
      return;
    }
    setFieldError(null);
    const success = active
      ? await onRevoke(reason)
      : await onActivate({
          procedure_reference: procedureReference,
          reason,
        });
    if (success) {
      setReason('');
      setProcedureReference('');
    }
  }

  return (
    <div className="rounded-md border border-border bg-muted/20 p-3">
      <div className="flex items-start gap-2">
        <div className="mt-0.5 flex size-7 items-center justify-center rounded-md bg-background text-muted-foreground">
          {active ? <ShieldAlert className="size-4" /> : <KeyRound className="size-4" />}
        </div>
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <p className="text-sm font-medium">Emergency access</p>
            {active ? (
              <Badge variant="destructive" className="rounded-md">
                Active
              </Badge>
            ) : (
              <Badge variant="outline" className="rounded-md">
                Inactive
              </Badge>
            )}
          </div>
          {active && user.break_glass ? (
            <div className="mt-3 space-y-2">
              <Detail label="Procedure" value={user.break_glass.procedure_reference} />
              <Detail label="Marked" value={formatDateTime(user.break_glass.created_at)} />
              <Detail label="Last used" value={formatDateTime(user.break_glass.last_used_at)} />
            </div>
          ) : null}
        </div>
      </div>

      {error ? (
        <Alert variant="destructive" className="mt-3">
          <Ban className="size-4" />
          <AlertTitle>Emergency access update failed</AlertTitle>
          <AlertDescription>{error.message}</AlertDescription>
        </Alert>
      ) : null}

      <div className="mt-3 flex flex-col gap-3">
        {!active ? (
          <Field>
            <FieldLabel htmlFor="break-glass-procedure">Procedure reference</FieldLabel>
            <Input
              id="break-glass-procedure"
              value={procedureReference}
              onChange={(event) => setProcedureReference(event.target.value)}
              placeholder="IR-2026-042"
            />
          </Field>
        ) : null}
        <Field>
          <FieldLabel htmlFor="break-glass-reason">Audit reason</FieldLabel>
          <Textarea
            id="break-glass-reason"
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            placeholder={
              active
                ? 'Why is emergency access being revoked?'
                : 'Why is this account being designated?'
            }
          />
          {fieldError ? <FieldError>{fieldError}</FieldError> : null}
        </Field>
        <Button
          type="button"
          variant={active ? 'outline' : 'destructive'}
          disabled={!canManageBreakGlass || pending}
          onClick={submit}
        >
          {active ? 'Revoke emergency access' : 'Mark as break-glass'}
        </Button>
        {!canManageBreakGlass ? (
          <p className="text-xs text-muted-foreground">
            Owner access is required to manage emergency accounts.
          </p>
        ) : null}
      </div>
    </div>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs font-medium uppercase text-muted-foreground">{label}</p>
      <p className="mt-1 break-words text-sm">{value}</p>
    </div>
  );
}

function formatDateTime(value: string | null | undefined): string {
  if (!value) {
    return 'Never';
  }
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(value));
}
