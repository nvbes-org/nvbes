import { useVirtualizer } from '@tanstack/react-virtual';
import { ArrowRight, ServerCog } from 'lucide-react';
import { useRef } from 'react';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { cn } from '@/lib/utils';
import type { ServiceAccount } from '../identity.service-accounts.api';
import { badgeVariantForStatus, statusLabel } from './WorkspaceServiceAccounts.helpers';

function EmptyState() {
  return (
    <div className="flex flex-col items-center gap-3 rounded-2xl border border-dashed border-border/70 px-6 py-10 text-center">
      <div className="flex size-12 items-center justify-center rounded-full bg-muted">
        <ServerCog className="size-5 text-muted-foreground" />
      </div>
      <div className="flex flex-col gap-1">
        <p className="text-sm font-medium">Aucun service account</p>
        <p className="text-sm text-muted-foreground">
          Creer le premier principal machine pour ce workspace.
        </p>
      </div>
    </div>
  );
}

export function WorkspaceServiceAccountsListCard({
  serviceAccounts,
  selectedServiceAccountId,
  onSelect,
}: {
  serviceAccounts: ServiceAccount[];
  selectedServiceAccountId: string;
  onSelect: (principalId: string) => void;
}) {
  const parentRef = useRef<HTMLDivElement>(null);
  const rowCount = serviceAccounts.length > 0 ? serviceAccounts.length * 2 - 1 : 0;

  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => parentRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 1 : 68),
    overscan: 8,
  });

  return (
    <Card className="min-h-[28rem]">
      <CardHeader>
        <CardTitle>Service accounts</CardTitle>
        <CardDescription>
          {serviceAccounts.length} principal{serviceAccounts.length > 1 ? 's' : ''} configure
          {serviceAccounts.length > 1 ? 's' : ''}.
        </CardDescription>
      </CardHeader>
      <CardContent ref={parentRef} className="max-h-[24rem] overflow-auto p-0">
        {serviceAccounts.length === 0 ? (
          <div className="px-6 pb-6">
            <EmptyState />
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
                    <div className="my-1 h-px bg-border/70" />
                  </div>
                );
              }

              const account = serviceAccounts[Math.floor(virtualItem.index / 2)];
              const selected = account.principal_id === selectedServiceAccountId;

              return (
                <div
                  key={account.principal_id}
                  ref={virtualizer.measureElement}
                  data-index={virtualItem.index}
                  className="absolute left-0 right-0 px-6"
                  style={{ transform: `translateY(${virtualItem.start}px)` }}
                >
                  <button
                    type="button"
                    className={cn(
                      'flex w-full items-center justify-between gap-3 rounded-xl px-3 py-3 text-left transition',
                      selected
                        ? 'bg-primary/10 text-primary'
                        : 'hover:bg-muted/60 hover:text-foreground',
                    )}
                    onClick={() => onSelect(account.principal_id)}
                  >
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="truncate text-sm font-medium">{account.name}</span>
                        <Badge variant={badgeVariantForStatus(account.status)}>
                          {statusLabel(account.status)}
                        </Badge>
                      </div>
                      <p className="mt-1 truncate text-xs text-muted-foreground">
                        {account.role} · {account.oauth_clients.length} client
                        {account.oauth_clients.length !== 1 ? 's' : ''}
                      </p>
                    </div>
                    <ArrowRight className="size-4 shrink-0 text-muted-foreground" />
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
