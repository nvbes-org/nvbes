import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import type { UseIdentityLinkedAppsPageResult } from '@/pages/useIdentityLinkedAppsPage';
import { LinkedAppsEmptyState } from './IdentityLinkedAppsPage.empty';
import { LinkedAppRow } from './IdentityLinkedAppsPage.row';

export function LinkedAppsList(props: {
  clients: UseIdentityLinkedAppsPageResult['clients'];
  listRef: UseIdentityLinkedAppsPageResult['listRef'];
  revoking: UseIdentityLinkedAppsPageResult['revoking'];
  virtualizer: UseIdentityLinkedAppsPageResult['virtualizer'];
  onRevoke: UseIdentityLinkedAppsPageResult['handleRevoke'];
}) {
  const { clients, listRef, revoking, virtualizer, onRevoke } = props;

  return (
    <IdentityPage>
      <IdentityPageHeader
        size="section"
        title="Apps liees"
        description="Gerer les applications et services connectes a votre compte."
      />

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
    </IdentityPage>
  );
}
import { IdentityPage, IdentityPageHeader } from '@/components/IdentityPage';
