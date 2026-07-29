import { formatDistanceToNow, parseISO } from 'date-fns';
import { fr } from 'date-fns/locale/fr';
import {
  CalendarDays,
  Clock,
  Gamepad2,
  Laptop,
  LogOut,
  MapPin,
  ShieldCheck,
  Smartphone,
  Tablet,
  Watch,
} from 'lucide-react';
import type { AccountSession } from '@nvbes/identity-client';
import { AsyncStateButton } from '@/components/AsyncStateButton';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import {
  type DeviceGroup,
  type DeviceType,
  formatActiveSessionCount,
  riskLabel,
  trustLabel,
} from './AccountSessionsPage.device';
import { BrowserIcon, OsIcon } from './AccountSessionsPage.icons';

function DeviceTypeIcon({ deviceType }: { deviceType: DeviceType }) {
  if (deviceType === 'mobile') return <Smartphone className="size-4 text-muted-foreground" />;
  if (deviceType === 'tablet') return <Tablet className="size-4 text-muted-foreground" />;
  if (deviceType === 'console') return <Gamepad2 className="size-4 text-muted-foreground" />;
  if (deviceType === 'wearable') return <Watch className="size-4 text-muted-foreground" />;
  return <Laptop className="size-4 text-muted-foreground" />;
}

function formatRelativeDate(iso: string): string {
  try {
    return formatDistanceToNow(parseISO(iso), { addSuffix: true, locale: fr });
  } catch {
    return iso;
  }
}

function formatAbsoluteDate(iso: string): string {
  try {
    return new Intl.DateTimeFormat('fr', {
      dateStyle: 'medium',
      timeStyle: 'short',
    }).format(parseISO(iso));
  } catch {
    return iso;
  }
}

function formatSessionLocation(session: AccountSession): string {
  if (session.geo_country_code) {
    try {
      return (
        new Intl.DisplayNames(['fr'], { type: 'region' }).of(session.geo_country_code) ??
        session.geo_country_code
      );
    } catch {
      return session.geo_country_code;
    }
  }

  if (session.ip === '127.0.0.1' || session.ip === '::1') {
    return 'Réseau local';
  }

  return 'Localisation indisponible';
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
  const { parsedDevice, sessions, isCurrentDevice } = device;
  const nonCurrentSessions = sessions.filter((s) => !s.current);
  const browserVersion = parsedDevice.browserVersion ? ` ${parsedDevice.browserVersion}` : '';
  const osVersion = parsedDevice.osVersion ? ` ${parsedDevice.osVersion}` : '';

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
            <span className="text-sm font-semibold truncate">
              {parsedDevice.browser}
              {browserVersion}
            </span>
            {parsedDevice.os && (
              <>
                <span className="text-muted-foreground">·</span>
                <OsIcon
                  osKey={parsedDevice.osKey}
                  className="size-3.5 text-muted-foreground shrink-0"
                />
                <span className="text-xs text-muted-foreground truncate">
                  {parsedDevice.os}
                  {osVersion}
                </span>
              </>
            )}
          </div>

          <div className="flex flex-wrap items-center gap-x-2 text-xs text-muted-foreground">
            {parsedDevice.device && <span>{parsedDevice.device}</span>}
            <span>{formatActiveSessionCount(sessions.length)}</span>
            {device.trustLevel && (
              <Badge variant="secondary" className="h-4 px-1.5 text-[10px]">
                <ShieldCheck />
                {trustLabel(device.trustLevel)}
                {device.trustScore !== null ? ` · ${device.trustScore}/100` : ''}
              </Badge>
            )}
          </div>
        </div>

        <div className="flex items-center gap-2 shrink-0">
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
                <MapPin className="size-3" />
                {formatSessionLocation(session)}
              </span>
              <span className="flex items-center gap-1">
                <Clock className="size-3" />
                Activité {formatRelativeDate(session.last_seen_at)}
              </span>
              <span
                className="flex items-center gap-1"
                title={formatAbsoluteDate(session.created_at)}
              >
                <CalendarDays className="size-3" />
                Créée {formatRelativeDate(session.created_at)}
              </span>
              <span title={formatAbsoluteDate(session.expires_at)}>
                Expire {formatRelativeDate(session.expires_at)}
              </span>
              {session.workspace_region && <span>· {session.workspace_region}</span>}
              {session.risk_decision && (
                <Badge
                  variant={session.risk_decision === 'deny' ? 'destructive' : 'outline'}
                  className="h-4 px-1.5 text-[10px]"
                >
                  {riskLabel(session.risk_decision)}
                  {session.risk_score !== null ? ` · ${Math.round(session.risk_score)}/100` : ''}
                </Badge>
              )}
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
