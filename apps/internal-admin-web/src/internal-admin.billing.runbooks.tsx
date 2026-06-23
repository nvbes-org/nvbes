import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { CheckCircle2, LifeBuoy, PlayCircle, RotateCcw } from 'lucide-react';
import { useState } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Skeleton } from '@/components/ui/skeleton';
import { Textarea } from '@/components/ui/textarea';
import { executeBillingRunbook, listBillingRunbooks } from './internal-admin.api';
import { LockedState } from './internal-admin.locked-state';
import type {
  AdminCredentials,
  BillingRunbook,
  RunbookExecutionResult,
} from './internal-admin.types';

export function BillingRunbooks({
  credentials,
  disabled,
}: {
  credentials: AdminCredentials;
  disabled: boolean;
}) {
  const queryClient = useQueryClient();
  const [activeRunbookId, setActiveRunbookId] = useState<string | null>(null);
  const [confirmCode, setConfirmCode] = useState('');
  const [reason, setReason] = useState('');
  const [result, setResult] = useState<RunbookExecutionResult | null>(null);
  const runbooks = useQuery({
    queryKey: ['billing-runbooks'],
    queryFn: () => listBillingRunbooks(credentials),
    enabled: !disabled,
  });
  const mutation = useMutation({
    mutationFn: (runbookId: string) =>
      executeBillingRunbook(credentials, runbookId, {
        confirm_code: confirmCode,
        reason,
      }),
    onSuccess: async (data) => {
      setResult(data);
      setConfirmCode('');
      setReason('');
      setActiveRunbookId(null);
      await Promise.all([
        queryClient.invalidateQueries({
          queryKey: ['internal-audit-events', credentials.workspaceId],
        }),
        queryClient.invalidateQueries({ queryKey: ['billing-runbooks'] }),
      ]);
    },
  });

  return (
    <section className="border-border bg-card rounded-lg border p-4" id="runbooks">
      <div className="mb-4 flex items-center gap-3">
        <div className="bg-secondary text-secondary-foreground flex size-9 items-center justify-center rounded-md">
          <LifeBuoy className="size-4" />
        </div>
        <div>
          <h2 className="text-sm font-semibold">Runbooks billing</h2>
          <p className="text-muted-foreground text-xs">
            Procedures operateur exposees par l'API interne.
          </p>
        </div>
      </div>
      <div className="space-y-3">
        {disabled ? (
          <LockedState label="Connecte un contexte operateur pour charger les runbooks internes." />
        ) : null}
        {runbooks.isLoading ? <Skeleton className="h-24 w-full" /> : null}
        {(runbooks.data ?? []).map((runbook) => (
          <RunbookCard
            disabled={disabled}
            error={activeRunbookId === runbook.id ? mutation.error : null}
            isActive={activeRunbookId === runbook.id}
            isPending={mutation.isPending && activeRunbookId === runbook.id}
            key={runbook.id}
            onCancel={() => {
              setActiveRunbookId(null);
              setConfirmCode('');
              setReason('');
            }}
            onConfirmCodeChange={setConfirmCode}
            onReasonChange={setReason}
            onRun={() => mutation.mutate(runbook.id)}
            onStart={() => {
              setResult(null);
              setActiveRunbookId(runbook.id);
            }}
            confirmCode={confirmCode}
            reason={reason}
            runbook={runbook}
          />
        ))}
        {runbooks.error ? (
          <p className="text-destructive text-xs">
            {runbooks.error instanceof Error ? runbooks.error.message : 'Runbooks unavailable'}
          </p>
        ) : null}
        {result ? (
          <div className="border-primary/20 bg-primary/5 text-primary rounded-md border p-3 text-xs">
            <div className="flex items-center gap-2 font-medium">
              <CheckCircle2 className="size-4" />
              {result.audit_action}
            </div>
            <p className="mt-1 font-mono">{result.runbook_id}</p>
          </div>
        ) : null}
      </div>
    </section>
  );
}

function RunbookCard({
  disabled,
  error,
  confirmCode,
  isActive,
  isPending,
  onCancel,
  onConfirmCodeChange,
  onReasonChange,
  onRun,
  onStart,
  reason,
  runbook,
}: {
  confirmCode: string;
  disabled: boolean;
  error: Error | null;
  isActive: boolean;
  isPending: boolean;
  onCancel: () => void;
  onConfirmCodeChange: (confirmCode: string) => void;
  onReasonChange: (reason: string) => void;
  onRun: () => void;
  onStart: () => void;
  reason: string;
  runbook: BillingRunbook;
}) {
  const isReasonReady = reason.trim().length >= 12;
  const expectedCode = 'EXECUTE RUNBOOK';
  const isConfirmationReady = confirmCode.trim() === expectedCode;

  return (
    <article className="bg-muted/40 rounded-md border p-3">
      <div className="mb-2 flex items-center justify-between gap-3">
        <h3 className="text-sm font-medium">{runbook.title}</h3>
        <Badge variant={runbook.severity === 'critical' ? 'destructive' : 'secondary'}>
          {runbook.severity}
        </Badge>
      </div>
      <ol className="text-muted-foreground list-decimal space-y-1 pl-4 text-xs">
        {runbook.steps.map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ol>

      {isActive ? (
        <div className="bg-background mt-3 grid gap-3 rounded-md border p-3">
          <Textarea
            disabled={disabled || isPending}
            onChange={(event) => onReasonChange(event.target.value)}
            placeholder="Motif audit, incident, ticket, approbation..."
            value={reason}
          />
          <Input
            className="font-mono text-xs"
            disabled={disabled || isPending}
            onChange={(event) => onConfirmCodeChange(event.target.value)}
            placeholder={expectedCode}
            value={confirmCode}
          />
          <div className="flex flex-wrap items-center justify-between gap-2">
            <p className="text-muted-foreground text-xs">
              Motif detaille et code exact {expectedCode} requis.
            </p>
            <div className="flex gap-2">
              <Button disabled={isPending} onClick={onCancel} type="button" variant="outline">
                <RotateCcw className="size-4" />
                Annuler
              </Button>
              <Button
                disabled={disabled || !isReasonReady || !isConfirmationReady || isPending}
                onClick={onRun}
                type="button"
              >
                <PlayCircle className="size-4" />
                Marquer execute
              </Button>
            </div>
          </div>
          {error ? (
            <p className="text-destructive text-xs">
              {error instanceof Error ? error.message : 'Execution impossible'}
            </p>
          ) : null}
        </div>
      ) : (
        <Button className="mt-3" disabled={disabled || isPending} onClick={onStart} type="button">
          <PlayCircle className="size-4" />
          Executer
        </Button>
      )}
    </article>
  );
}
