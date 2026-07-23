import { Smartphone, X } from 'lucide-react';
import type { RefObject } from 'react';
import type { VirtualItem } from '@tanstack/react-virtual';
import type { AccountSession } from '@nvbes/identity-client';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { deviceTrustLabel, parseUserAgent } from './AccountSessionsPage.device';

export function OtherSessionsCard({
  sessions,
  listRef,
  totalSize,
  virtualItems,
  measureElement,
  revoking,
  onRevoke,
  onRevokeOthers,
}: {
  sessions: AccountSession[];
  listRef: RefObject<HTMLDivElement | null>;
  totalSize: number;
  virtualItems: VirtualItem[];
  measureElement: (element: Element | null) => void;
  revoking: string | null;
  onRevoke: (sessionId: string) => void;
  onRevokeOthers: () => void;
}) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle>Autres sessions</CardTitle>
            <CardDescription>
              {sessions.length} session(s) active(s) sur d&apos;autres appareils.
            </CardDescription>
          </div>
          <Button variant="outline" size="sm" onClick={onRevokeOthers} className="shrink-0">
            Deconnecter les autres
          </Button>
        </div>
      </CardHeader>
      <CardContent ref={listRef} className="max-h-[32rem] overflow-auto p-0">
        <div
          style={{
            height: `${totalSize}px`,
            position: 'relative',
          }}
        >
          {virtualItems.map((virtualItem) => {
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

            const session = sessions[Math.floor(virtualItem.index / 2)];
            const device = session.user_agent
              ? parseUserAgent(session.user_agent)
              : { browser: 'Appareil inconnu', os: '' };

            return (
              <div
                key={session.id}
                ref={measureElement}
                data-index={virtualItem.index}
                className="absolute left-0 right-0 px-6"
                style={{ transform: `translateY(${virtualItem.start}px)` }}
              >
                <div className="flex items-center gap-3 py-1">
                  <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
                    <Smartphone className="size-4 text-muted-foreground" />
                  </div>
                  <div className="flex min-w-0 flex-1 flex-col">
                    <span className="text-sm font-medium">{device.browser}</span>
                    <span className="text-xs text-muted-foreground">
                      {device.os}
                      {session.ip ? ` · ${session.ip}` : ''}
                    </span>
                  </div>
                  <Badge variant="secondary" className="shrink-0">
                    {deviceTrustLabel(session.device_trust_level)}
                  </Badge>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    disabled={revoking === session.id}
                    onClick={() => onRevoke(session.id)}
                  >
                    <X className="size-4" />
                    <span className="sr-only">Revoquer</span>
                  </Button>
                </div>
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
