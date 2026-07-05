import { useVirtualizer } from '@tanstack/react-virtual';
import { useRef } from 'react';

import { Separator } from '@/components/ui/separator';
import type { ServiceAccount, ServiceAccountClient } from '../identity.service-accounts.api';
import { OAuthClientRow } from './WorkspaceServiceAccountsDetailCard.oauth.row';
import { EmptyClientState } from './WorkspaceServiceAccountsDetailCard.oauth.shared';

export function OAuthClientsList({
  selectedServiceAccount,
  onRotateClientSecret,
  onRevokeClient,
}: {
  selectedServiceAccount: ServiceAccount;
  onRotateClientSecret: (client: ServiceAccountClient) => void;
  onRevokeClient: (client: ServiceAccountClient) => void;
}) {
  const clientsRef = useRef<HTMLDivElement>(null);
  const clientRowCount =
    selectedServiceAccount.oauth_clients.length > 0
      ? selectedServiceAccount.oauth_clients.length * 2 - 1
      : 0;
  const clientVirtualizer = useVirtualizer({
    count: clientRowCount,
    getScrollElement: () => clientsRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 12 : 148),
    overscan: 6,
  });

  return (
    <div ref={clientsRef} className="mt-4 max-h-[28rem] overflow-auto p-0">
      {selectedServiceAccount.oauth_clients.length === 0 ? (
        <EmptyClientState />
      ) : (
        <div
          style={{
            height: `${clientVirtualizer.getTotalSize()}px`,
            position: 'relative',
          }}
        >
          {clientVirtualizer.getVirtualItems().map((virtualItem) => {
            if (virtualItem.index % 2 === 1) {
              return (
                <div
                  key={`separator-${virtualItem.index}`}
                  className="absolute left-0 right-0"
                  style={{ transform: `translateY(${virtualItem.start}px)` }}
                >
                  <Separator className="my-2" />
                </div>
              );
            }

            const client = selectedServiceAccount.oauth_clients[Math.floor(virtualItem.index / 2)];

            return (
              <div
                key={client.id}
                ref={clientVirtualizer.measureElement}
                data-index={virtualItem.index}
                className="absolute left-0 right-0"
                style={{ transform: `translateY(${virtualItem.start}px)` }}
              >
                <OAuthClientRow
                  client={client}
                  onRotateClientSecret={onRotateClientSecret}
                  onRevokeClient={onRevokeClient}
                />
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
