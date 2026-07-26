import { formatDistanceToNow, parseISO } from 'date-fns';
import { fr } from 'date-fns/locale/fr';
import { Clock, Laptop, LogOut, Smartphone, Tablet } from 'lucide-react';
import type { AccountSession } from '@nvbes/identity-client';
import { AsyncStateButton } from '@/components/AsyncStateButton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { type DeviceGroup, deviceTrustLabel } from './AccountSessionsPage.device';
import { BrowserIcon, OsIcon } from './AccountSessionsPage.icons';

function DeviceTypeIcon({ deviceType }: { deviceType: 'desktop' | 'mobile' | 'tablet' }) {
  if (deviceType === 'mobile') return <Smartphone className="size-4 text-muted-foreground" />;
  if (deviceType === 'tablet') return <Tablet className="size-4 text-muted-foreground" />;
  return <Laptop className="size-4 text-muted-foreground" />;
}

function formatRelativeDate(iso: string): string {
  try {
    return formatDistanceToNow(parseISO(iso), { addSuffix: true, locale: fr });
  } catch {
    return iso;
  }
}

function trustVariant(level: string | null): 'default' | 'secondary' | 'outline' | 'destructive' {
  if (level === 'trusted') return 'default';
  if (level === 'recognized') return 'secondary';
  if (level === 'restricted') return 'destructive';
  return 'outline';
}

export function DeviceCard({
  device,
  revoking,
  onRevoke,
  onRevokeDevice,
}: {
  device: DeviceGroup;
  revoking: string | null;
  onRevoke: (sessionId: string) => void;
  onRevokeDevice?: (device: DeviceGroup) => void;
}) {
  const { parsedDevice, trustLevel, sessions, isCurrentDevice } = device;
  const nonCurrentSessions = sessions.filter((s) => !s.current);

  return (
    <Card className="overflow-hidden">
      {/* Device Header */}
      <div className="flex items-center gap-3 p-4 bg-card">
        <div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted">
          <DeviceTypeIcon deviceType={parsedDevice.deviceType} />
        </div>

        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <div className="flex items-center gap-2">
            <BrowserIcon
              browserKey={parsedDevice.browserKey}
              className="size-4 text-foreground shrink-0"
            />
            <span className="text-sm font-semibold truncate">{parsedDevice.browser}</span>
            {parsedDevice.os && (
              <>
                <span className="text-muted-foreground">·</span>
                <OsIcon
                  osKey={parsedDevice.osKey}
                  className="size-3.5 text-muted-foreground shrink-0"
                />
                <span className="text-xs text-muted-foreground truncate">{parsedDevice.os}</span>
              </>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
            <span>{sessions.length} session(s) active(s)</span>
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <Badge variant={trustVariant(trustLevel)}>{deviceTrustLabel(trustLevel)}</Badge>

          {!isCurrentDevice && onRevokeDevice && nonCurrentSessions.length > 1 && (
            <Button
              variant="outline"
              size="sm"
              className="text-xs text-destructive hover:text-destructive"
              onClick={() => onRevokeDevice(device)}
            >
              Révoquer tout
            </Button>
          )}
        </div>
      </div>

      <Separator />

      {/* Sessions list under device */}
      <CardContent className="p-0 divide-y divide-border">
        {sessions.map((session: AccountSession) => (
          <div
            key={session.id}
            className="flex items-center justify-between gap-3 px-4 py-3 text-xs text-muted-foreground hover:bg-muted/30 transition-colors"
          >
            <div className="flex flex-wrap items-center gap-x-3 gap-y-1 min-w-0">
              {session.current && (
                <Badge variant="default" className="text-[10px] px-1.5 py-0">
                  Session actuelle
                </Badge>
              )}
              {session.ip && <span className="font-mono">{session.ip}</span>}
              <span className="flex items-center gap-1">
                <Clock className="size-3" />
                {formatRelativeDate(session.last_seen_at)}
              </span>
              {session.workspace_region && <span>· {session.workspace_region}</span>}
            </div>

            {!session.current && (
              <AsyncStateButton
                type="button"
                variant="destructive"
                size="icon-sm"
                state={revoking === session.id ? 'pending' : 'idle'}
                message="Révoquer cette session"
                icon={<LogOut className="size-4" />}
                onClick={() => onRevoke(session.id)}
              />
            )}
          </div>
        ))}
      </CardContent>
    </Card>
  );
}
