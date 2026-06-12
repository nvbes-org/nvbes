import { AlertCircle, Clock3 } from 'lucide-react';

type EnterpriseRouteStateTone = 'neutral' | 'warning';

export type EnterpriseRouteStateProps = {
  title: string;
  description: string;
  tone?: EnterpriseRouteStateTone;
};

export function EnterpriseRouteState({
  title,
  description,
  tone = 'neutral',
}: EnterpriseRouteStateProps) {
  const Icon = tone === 'warning' ? AlertCircle : Clock3;

  return (
    <section className="flex min-h-28 items-start gap-3 rounded-lg border border-dashed border-border bg-muted/30 p-4">
      <div
        className={
          tone === 'warning'
            ? 'flex size-9 shrink-0 items-center justify-center rounded-md bg-destructive/10 text-destructive'
            : 'flex size-9 shrink-0 items-center justify-center rounded-md bg-primary/10 text-primary'
        }
      >
        <Icon className="size-4" />
      </div>
      <div className="min-w-0">
        <h2 className="text-sm font-heading font-semibold">{title}</h2>
        <p className="mt-1 max-w-2xl text-sm leading-6 text-muted-foreground">{description}</p>
      </div>
    </section>
  );
}
