import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import type { UseAccountLinkedAppsPageResult } from '@/pages/useAccountLinkedAppsPage';
import { LinkedAppsEmptyState } from './AccountLinkedAppsPage.empty';
import { LinkedAppRow } from './AccountLinkedAppsPage.row';

export function LinkedAppsList(props: {
  clients: UseAccountLinkedAppsPageResult['clients'];
  listRef: UseAccountLinkedAppsPageResult['listRef'];
  revoking: UseAccountLinkedAppsPageResult['revoking'];
  virtualizer: UseAccountLinkedAppsPageResult['virtualizer'];
  onRevoke: UseAccountLinkedAppsPageResult['handleRevoke'];
}) {
  const { clients, listRef, revoking, virtualizer, onRevoke } = props;

  return (
    <div className="flex animate-fade-slide-up flex-col gap-6 [animation-delay:0ms]">
      <div>
        <h1 className="font-heading text-xl font-semibold">Apps liees</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Gerer les applications et services connectes a votre compte.
        </p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Applications OAuth</CardTitle>
          <CardDescription>
            Applications tierces autorisees a acceder a votre compte. Revoguez l&apos;acces pour les
            applications que vous n&apos;utilisez plus.
          </CardDescription>
        </CardHeader>
        <CardContent ref={listRef} className="max-h-[32rem] overflow-auto p-0">
          {clients.length === 0 ? (
            <LinkedAppsEmptyState />
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

                const client = clients[Math.floor(virtualItem.index / 2)];

                return (
                  <div
                    key={client.id}
                    ref={virtualizer.measureElement}
                    data-index={virtualItem.index}
                    className="absolute left-0 right-0 px-6"
                    style={{ transform: `translateY(${virtualItem.start}px)` }}
                  >
                    <LinkedAppRow client={client} revoking={revoking} onRevoke={onRevoke} />
                  </div>
                );
              })}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
