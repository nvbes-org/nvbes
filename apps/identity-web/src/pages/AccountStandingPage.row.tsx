import { Badge } from '@/components/ui/badge';

export function formatStandingDate(date: string, options: Intl.DateTimeFormatOptions) {
  return new Date(date).toLocaleDateString('fr-FR', options);
}

export function StandingRow(props: {
  title: string;
  subtitle: string;
  badgeLabel: string;
  badgeVariant: 'default' | 'secondary' | 'outline';
  icon: React.ComponentType<{ className?: string }>;
  iconClassName?: string;
}) {
  const { title, subtitle, badgeLabel, badgeVariant, icon: Icon, iconClassName } = props;

  return (
    <div className="flex items-center justify-between gap-3 py-1">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
          <Icon className={iconClassName ?? 'size-4 text-muted-foreground'} />
        </div>
        <div className="flex min-w-0 flex-col">
          <span className="text-sm font-medium">{title}</span>
          <span className="text-xs text-muted-foreground">{subtitle}</span>
        </div>
      </div>
      <Badge variant={badgeVariant}>{badgeLabel}</Badge>
    </div>
  );
}
