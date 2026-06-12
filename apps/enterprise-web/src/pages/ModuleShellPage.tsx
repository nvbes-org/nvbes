import { EnterpriseRouteState } from '../components/EnterpriseRouteState';

export type ModuleShellPageProps = {
  title: string;
  summary: string;
  metrics: Array<{ label: string; value: string; tone?: 'neutral' | 'warning' }>;
};

export function ModuleShellPage({ title, summary, metrics }: ModuleShellPageProps) {
  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-col gap-2">
        <p className="text-xs font-semibold uppercase tracking-[0.16em] text-muted-foreground">
          Enterprise module
        </p>
        <div className="flex flex-col gap-2 md:flex-row md:items-end md:justify-between">
          <div>
            <h1 className="text-2xl font-heading font-semibold tracking-normal">{title}</h1>
            <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">{summary}</p>
          </div>
          <div className="h-9 shrink-0 rounded-md border border-border px-3 py-2 text-xs font-medium text-muted-foreground">
            Shell
          </div>
        </div>
      </header>

      <section className="grid gap-3 md:grid-cols-3">
        {metrics.map((metric) => (
          <div key={metric.label} className="min-h-28 rounded-lg border border-border bg-card p-4">
            <p className="truncate text-xs font-medium text-muted-foreground">{metric.label}</p>
            <p
              className={
                metric.tone === 'warning'
                  ? 'mt-5 truncate text-xl font-heading font-semibold text-destructive'
                  : 'mt-5 truncate text-xl font-heading font-semibold'
              }
            >
              {metric.value}
            </p>
          </div>
        ))}
      </section>

      <EnterpriseRouteState
        tone={metrics.some((metric) => metric.tone === 'warning') ? 'warning' : 'neutral'}
        title={`${title} workflow pending`}
        description="This route is intentionally limited to a stable navigation shell until its dedicated workflow task adds data loading, mutations, and backend integration."
      />
    </div>
  );
}
