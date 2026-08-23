import { AlertCircle, Clock3 } from 'lucide-react';
import { Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle } from './ui/empty';

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
    <Empty className="min-h-28 items-start justify-start border border-dashed border-border bg-muted/30 text-left">
      <div className="flex items-start gap-3">
        <EmptyMedia
          variant="icon"
          className={
            tone === 'warning' ? 'bg-destructive/10 text-destructive' : 'bg-primary/10 text-primary'
          }
        >
          <Icon className="size-4" />
        </EmptyMedia>
        <EmptyHeader className="max-w-2xl items-start gap-1">
          <EmptyTitle>{title}</EmptyTitle>
          <EmptyDescription>{description}</EmptyDescription>
        </EmptyHeader>
      </div>
    </Empty>
  );
}
