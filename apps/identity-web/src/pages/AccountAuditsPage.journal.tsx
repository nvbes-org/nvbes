import { useVirtualizer } from '@tanstack/react-virtual';
import { AlertTriangle, FileSearch, Shield } from 'lucide-react';
import { useRef } from 'react';
import type { AccountMe } from '@nvbes/identity-client';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { eventLabels, type SecurityEvent } from './AccountAuditsPage.api';

function AuditEventRow({ event }: { event: SecurityEvent }) {
  const label = eventLabels[event.event_type] ?? event.event_type;
  const date = new Date(event.created_at).toLocaleDateString('fr-FR', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });

  return (
    <div className="flex items-center justify-between gap-3 py-1">
      <div className="flex min-w-0 items-center gap-3">
        <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
          {event.event_type.includes('failed') || event.event_type.includes('error') ? (
            <AlertTriangle className="size-4 text-destructive/70" />
          ) : (
            <Shield className="size-4 text-muted-foreground" />
          )}
        </div>
        <div className="flex min-w-0 flex-col">
          <span className="text-sm font-medium">{label}</span>
          <span className="text-xs text-muted-foreground">
            {date}
            {event.ip_address ? ` · ${event.ip_address}` : ''}
          </span>
        </div>
      </div>
      <Badge variant="secondary" className="shrink-0">
        {event.status ?? 'success'}
      </Badge>
    </div>
  );
}

export function SecurityJournalCard({
  me,
  events,
}: {
  me: AccountMe | null;
  events: SecurityEvent[];
}) {
  const listRef = useRef<HTMLDivElement>(null);
  const rowCount = events.length > 0 ? events.length * 2 - 1 : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 8 : 56),
    overscan: 10,
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>Journal de securite</CardTitle>
        <CardDescription>
          Evenements de securite recents pour votre workspace actuel.
          {events.length > 0 && ` (${events.length} evenements)`}
        </CardDescription>
      </CardHeader>
      <CardContent ref={listRef} className="max-h-[32rem] overflow-auto p-0">
        {events.length === 0 ? (
          <div className="px-6 py-8">
            <div className="flex flex-col items-center gap-3">
              <FileSearch className="size-8 text-muted-foreground" />
              <div className="text-center">
                <p className="text-sm text-muted-foreground">
                  {me?.current_workspace_id
                    ? 'Aucun evenement de securite disponible.'
                    : 'Selectionnez un workspace pour voir les evenements de securite.'}
                </p>
              </div>
            </div>
          </div>
        ) : (
          <div
            style={{
              height: `${virtualizer.getTotalSize()}px`,
              position: 'relative',
            }}
          >
            {virtualizer.getVirtualItems().map((virtualItem) => {
              if (virtualItem.index % 2 === 1) {
                return (
                  <div
                    key={`separator-${virtualItem.index}`}
                    className="absolute left-0 right-0 px-6"
                    style={{ transform: `translateY(${virtualItem.start}px)` }}
                  >
                    <Separator className="my-1" />
                  </div>
                );
              }

              const event = events[Math.floor(virtualItem.index / 2)];

              return (
                <div
                  key={event.id}
                  ref={virtualizer.measureElement}
                  data-index={virtualItem.index}
                  className="absolute left-0 right-0 px-6"
                  style={{ transform: `translateY(${virtualItem.start}px)` }}
                >
                  <AuditEventRow event={event} />
                </div>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
