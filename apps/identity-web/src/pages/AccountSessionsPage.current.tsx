import { Laptop } from 'lucide-react';
import type { AccountSession } from '@nvbes/identity-client';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { parseUserAgent } from './AccountSessionsPage.device';

export function CurrentSessionCard({ session }: { session: AccountSession }) {
  const device = session.user_agent
    ? parseUserAgent(session.user_agent)
    : { browser: 'Appareil inconnu', os: '' };

  return (
    <Card>
      <CardHeader>
        <CardTitle>Session actuelle</CardTitle>
        <CardDescription>L&apos;appareil que vous utilisez actuellement.</CardDescription>
      </CardHeader>
      <CardContent>
        <div className="flex items-center gap-3">
          <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-primary/10">
            <Laptop className="size-4 text-primary" />
          </div>
          <div className="flex min-w-0 flex-col">
            <span className="text-sm font-medium">{device.browser}</span>
            <span className="text-xs text-muted-foreground">
              {device.os}
              {session.ip ? ` · ${session.ip}` : ''}
            </span>
          </div>
          <Badge variant="default" className="ml-auto shrink-0">
            Actuel
          </Badge>
        </div>
      </CardContent>
    </Card>
  );
}
