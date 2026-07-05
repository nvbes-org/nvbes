import { useVirtualizer } from '@tanstack/react-virtual';
import { ArrowRight, ServerCog } from 'lucide-react';
import { useRef } from 'react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '@/components/ui/empty';
import { Separator } from '@/components/ui/separator';
import { cn } from '@/lib/classnames';
import type { ServiceAccount } from '../identity.service-accounts.api';
import { badgeVariantForStatus, statusLabel } from './WorkspaceServiceAccounts.helpers';

function EmptyState() {
  return (
    <Empty className="border">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <ServerCog />
        </EmptyMedia>
        <EmptyTitle>Aucun service account</EmptyTitle>
        <EmptyDescription>Creer le premier principal machine pour ce workspace.</EmptyDescription>
      </EmptyHeader>
    </Empty>
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
                  <Separator
                    key={`separator-${virtualItem.index}`}
                    className="absolute left-6 right-6 w-auto"
                    style={{ transform: `translateY(${virtualItem.start}px)` }}
                  />
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
                  <Button
                    type="button"
                    variant="ghost"
                    className={cn(
                      'h-auto w-full justify-between gap-3 px-3 py-3 text-left',
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
                  </Button>
                </div>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
