import { CheckCircle2, LockKeyhole } from 'lucide-react';
import { Button } from '@/components/ui/button';

export function StatusStrip({ isReady, readiness }: { isReady: boolean; readiness: number }) {
  return (
    <section className="border-border bg-card mb-5 flex flex-col gap-3 rounded-lg border p-4 md:flex-row md:items-center md:justify-between">
      <div className="flex items-start gap-3">
        <div className="bg-primary/10 text-primary flex size-9 shrink-0 items-center justify-center rounded-md">
          {isReady ? <CheckCircle2 className="size-4" /> : <LockKeyhole className="size-4" />}
        </div>
        <div>
          <h2 className="text-sm font-semibold">
            {isReady ? 'Cockpit operationnel' : 'Cockpit verrouille'}
          </h2>
          <p className="text-muted-foreground text-xs">
            {isReady
              ? 'Les recherches, exports, audits et mutations utilisent le contexte courant.'
              : 'Complete le workspace, le token interne et l acteur avant toute operation.'}
          </p>
        </div>
      </div>
      <div className="flex items-center gap-3">
        <div className="bg-muted h-2 w-32 overflow-hidden rounded-full">
          <div className="bg-primary h-full rounded-full" style={{ width: `${readiness}%` }} />
        </div>
        <Button asChild size="sm" variant={isReady ? 'outline' : 'default'}>
          <a href="#contexte">Contexte {readiness}%</a>
        </Button>
      </div>
    </section>
  );
}
