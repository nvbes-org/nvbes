import {
  AlertTriangle,
  CheckCircle2,
  Clock3,
  KeyRound,
  ShieldAlert,
  ShieldCheck,
} from 'lucide-react';
import { Badge } from '../components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import {
  calculateSecurityPostureScore,
  type SecurityPostureControl,
  type SecurityPostureStatus,
  tenantSecurityControls,
} from '../enterprise.security-posture';
import { cn } from '../lib/utils';

const posture = calculateSecurityPostureScore(tenantSecurityControls);

const statusLabels: Record<SecurityPostureStatus, string> = {
  complete: 'Complete',
  attention: 'A traiter',
  critical: 'Critique',
};

const statusClasses: Record<SecurityPostureStatus, string> = {
  complete: 'border-emerald-200 bg-emerald-50 text-emerald-700',
  attention: 'border-amber-200 bg-amber-50 text-amber-700',
  critical: 'border-red-200 bg-red-50 text-red-700',
};

export function SecurityPage() {
  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-2xl font-heading font-semibold">Security</h1>
            <Badge variant="outline" className="rounded-md">
              V0
            </Badge>
          </div>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
            Tenant security posture score with prioritized remediation for identity, sessions,
            domain trust, SSO, and developer secrets.
          </p>
        </div>
        <Badge className={cn('h-7 shrink-0 rounded-md border', statusClasses[posture.status])}>
          {statusLabels[posture.status]}
        </Badge>
      </header>

      <section className="grid gap-4 lg:grid-cols-[320px_minmax(0,1fr)]">
        <Card className="rounded-lg" size="sm">
          <CardContent>
            <div className="flex items-start justify-between gap-3">
              <div>
                <p className="text-xs font-medium text-muted-foreground">Posture score</p>
                <p className="mt-3 text-5xl font-heading font-semibold tracking-normal">
                  {posture.score}
                </p>
              </div>
              <div className="rounded-lg border border-red-200 bg-red-50 p-2 text-red-700">
                <ShieldAlert className="size-5" />
              </div>
            </div>
            <div className="mt-5 h-2 overflow-hidden rounded-full bg-muted">
              <div
                className="h-full rounded-full bg-primary"
                style={{ width: `${posture.score}%` }}
              />
            </div>
            <div className="mt-4 grid grid-cols-2 gap-3 text-sm">
              <Metric
                label="Controls complete"
                value={`${posture.completedControls}/${posture.totalControls}`}
              />
              <Metric
                label="Weighted points"
                value={`${posture.completedWeight}/${posture.maxScore}`}
              />
            </div>
          </CardContent>
        </Card>

        <Card className="rounded-lg" size="sm">
          <CardHeader className="border-b border-border">
            <CardTitle>Concrete recommendations</CardTitle>
          </CardHeader>
          <CardContent className="divide-y divide-border px-0">
            {posture.openRecommendations.map((control) => (
              <RecommendationRow key={control.id} control={control} />
            ))}
          </CardContent>
        </Card>
      </section>

      <section className="grid gap-3 md:grid-cols-2 xl:grid-cols-5">
        {tenantSecurityControls.map((control) => (
          <ControlCard key={control.id} control={control} />
        ))}
      </section>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return (
    <div className="rounded-lg border border-border bg-muted/20 p-3">
      <p className="truncate text-xs text-muted-foreground">{label}</p>
      <p className="mt-1 truncate text-sm font-semibold">{value}</p>
    </div>
  );
}

function RecommendationRow({ control }: { control: SecurityPostureControl }) {
  const Icon = control.status === 'critical' ? AlertTriangle : Clock3;

  return (
    <div className="flex min-h-20 items-start gap-3 px-4 py-3">
      <div className={cn('rounded-md border p-1.5', statusClasses[control.status])}>
        <Icon className="size-4" />
      </div>
      <div className="min-w-0 flex-1">
        <div className="flex flex-wrap items-center gap-2">
          <p className="text-sm font-semibold">{control.recommendation}</p>
          <Badge variant="secondary" className="rounded-md">
            {control.owner}
          </Badge>
        </div>
        <p className="mt-1 text-sm leading-5 text-muted-foreground">{control.description}</p>
      </div>
    </div>
  );
}

function ControlCard({ control }: { control: SecurityPostureControl }) {
  const Icon =
    control.status === 'complete'
      ? CheckCircle2
      : control.id === 'old-secrets'
        ? KeyRound
        : ShieldCheck;
  const completion = Math.round((control.completedWeight / control.weight) * 100);

  return (
    <Card className="min-h-36 rounded-lg" size="sm">
      <CardContent>
        <div className="flex items-start justify-between gap-2">
          <Icon className="size-4 shrink-0 text-muted-foreground" />
          <Badge className={cn('rounded-md border', statusClasses[control.status])}>
            {statusLabels[control.status]}
          </Badge>
        </div>
        <p className="mt-4 truncate text-sm font-semibold">{control.label}</p>
        <p className="mt-1 line-clamp-2 text-xs leading-5 text-muted-foreground">
          {control.description}
        </p>
        <div className="mt-4 h-1.5 overflow-hidden rounded-full bg-muted">
          <div className="h-full rounded-full bg-primary" style={{ width: `${completion}%` }} />
        </div>
      </CardContent>
    </Card>
  );
}
