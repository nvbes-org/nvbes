import type { UserConsent } from '@nvbes/identity-client';
import { useVirtualizer } from '@tanstack/react-virtual';
import { useRef } from 'react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import {
  ConsentEmptyState,
  ConsentRow,
  consentLabels,
  isVisibleConsentType,
} from './AccountPrivacyPage.shared';

export function AccountPrivacyConsentList({
  consents,
  onRevoke,
}: {
  consents: UserConsent[];
  onRevoke: (consent: UserConsent) => void;
}) {
  const visibleConsents = consents.filter((consent) => isVisibleConsentType(consent.consent_type));
  const listRef = useRef<HTMLDivElement>(null);
  const rowCount = visibleConsents.length > 0 ? visibleConsents.length * 2 - 1 : 0;
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => listRef.current,
    estimateSize: (index) => (index % 2 === 1 ? 8 : 56),
    overscan: 8,
  });

  return (
    <Card>
      <CardHeader>
        <CardTitle>Consentements actifs</CardTitle>
        <CardDescription>
          Les consentements que vous avez accordes. Vous pouvez les revoquer a tout moment.
        </CardDescription>
      </CardHeader>
      <CardContent ref={listRef} className="max-h-[32rem] overflow-auto p-0">
        {visibleConsents.length === 0 ? (
          <ConsentEmptyState />
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
                    style={{
                      transform: `translateY(${virtualItem.start}px)`,
                    }}
                  >
                    <Separator className="my-1" />
                  </div>
                );
              }

              const consent = visibleConsents[Math.floor(virtualItem.index / 2)];
              const label = consentLabels[consent.consent_type] ?? consent.consent_type;

              return (
                <div
                  key={consent.id}
                  ref={virtualizer.measureElement}
                  data-index={virtualItem.index}
                  className="absolute left-0 right-0 px-6"
                  style={{ transform: `translateY(${virtualItem.start}px)` }}
                >
                  <ConsentRow consent={consent} label={label} onRevoke={() => onRevoke(consent)} />
                </div>
              );
            })}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
