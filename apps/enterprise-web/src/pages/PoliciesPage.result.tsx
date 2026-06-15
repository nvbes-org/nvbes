import type { EnterprisePolicySimulationResponse } from '@nvbes/identity-client';
import { AlertCircle, CheckCircle2, ShieldCheck, XCircle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '../components/ui/alert';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import { Skeleton } from '../components/ui/skeleton';

export function SimulationResult({
  result,
  error,
  pending,
}: {
  result: EnterprisePolicySimulationResponse | null;
  error: Error | null;
  pending: boolean;
}) {
  if (pending) {
    return <Skeleton className="h-72 w-full rounded-lg" />;
  }

  if (error) {
    return (
      <Alert variant="destructive">
        <AlertCircle className="size-4" />
        <AlertTitle>Simulation failed</AlertTitle>
        <AlertDescription>{error.message}</AlertDescription>
      </Alert>
    );
  }

  if (!result) {
    return (
      <Card className="rounded-lg" size="sm">
        <CardContent className="flex min-h-72 flex-col items-center justify-center gap-3 text-center">
          <ShieldCheck className="size-8 text-muted-foreground" />
          <p className="text-sm font-medium">No simulation run yet</p>
          <p className="max-w-64 text-sm text-muted-foreground">
            Select a subject, workspace, and action to inspect the effective decision.
          </p>
        </CardContent>
      </Card>
    );
  }

  const decision = result.decision;
  return (
    <Card className="rounded-lg" size="sm">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          {decision.allowed ? (
            <CheckCircle2 className="size-4 text-emerald-600" />
          ) : (
            <XCircle className="size-4 text-destructive" />
          )}
          {decision.allowed ? 'Allowed' : 'Denied'}
        </CardTitle>
      </CardHeader>
      <CardContent className="grid gap-3 text-sm">
        <Detail label="Reason" value={decision.reason} />
        <Detail label="Subject" value={`${decision.subject_label} (${decision.subject_type})`} />
        <Detail label="Role" value={decision.role ?? 'No active workspace role'} />
        <Detail label="Action" value={decision.action} />
        <Detail
          label="Step-up"
          value={decision.requires_step_up ? 'Required for users' : 'Not required'}
        />
      </CardContent>
    </Card>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-start justify-between gap-3 border-b pb-2 last:border-b-0 last:pb-0">
      <span className="text-muted-foreground">{label}</span>
      <span className="max-w-44 text-right font-medium break-words">{value}</span>
    </div>
  );
}
