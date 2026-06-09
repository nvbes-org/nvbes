import { Calendar, Mail, MapPin, User } from 'lucide-react';
import type { AccountPrincipal } from '@nvbes/identity-client';

import { Alert, AlertDescription } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Separator } from '@/components/ui/separator';
import { Skeleton } from '@/components/ui/skeleton';

export function PersonalInfoSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <div>
        <Skeleton className="mt-1 h-4 w-72" />
        <Skeleton className="h-6 w-48" />
      </div>
      <Card>
        <CardContent className="flex flex-col gap-3 pt-6">
          {Array.from({ length: 5 }).map((_, index) => (
            <Skeleton key={index} className="h-10 w-full rounded-lg" />
          ))}
        </CardContent>
      </Card>
    </div>
  );
}

function Message({ message, tone }: { message: string; tone: 'error' | 'success' }) {
  return (
    <Alert variant={tone === 'error' ? 'destructive' : 'default'}>
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  );
}

export function PersonalInfoError({ message }: { message: string }) {
  return <Message message={message} tone="error" />;
}

export function PersonalInfoSuccess({ message }: { message: string }) {
  return <Message message={message} tone="success" />;
}

export function PersonalInfoSummary({
  user,
  fullName,
  memberSince,
}: {
  user: AccountPrincipal;
  fullName: string;
  memberSince: string;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Profil</CardTitle>
        <CardDescription>Vos informations d&apos;identite</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-1">
        <div className="flex items-center gap-3 py-1">
          <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
            <User className="size-4 text-muted-foreground" />
          </div>
          <div className="flex min-w-0 flex-col">
            <span className="text-xs text-muted-foreground">Nom complet</span>
            <span className="truncate text-sm font-medium">{fullName}</span>
          </div>
        </div>
        <Separator className="my-1" />
        <div className="flex items-center gap-3 py-1">
          <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
            <Mail className="size-4 text-muted-foreground" />
          </div>
          <div className="flex min-w-0 flex-col">
            <span className="text-xs text-muted-foreground">Email</span>
            <span className="truncate text-sm font-medium">{user.email}</span>
          </div>
        </div>
        <div className="flex items-center justify-between py-1">
          <span className="text-sm font-medium text-muted-foreground">Statut</span>
          <Badge variant={user.email_verified ? 'default' : 'secondary'}>
            {user.email_verified ? 'Verifie' : 'En attente'}
          </Badge>
        </div>
        <Separator className="my-1" />
        <div className="flex items-center gap-3 py-1">
          <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
            <MapPin className="size-4 text-muted-foreground" />
          </div>
          <div className="flex min-w-0 flex-col">
            <span className="text-xs text-muted-foreground">Region</span>
            <span className="truncate text-sm font-medium">{user.region ?? 'Non definie'}</span>
          </div>
        </div>
        <Separator className="my-1" />
        <div className="flex items-center gap-3 py-1">
          <div className="flex size-8 shrink-0 items-center justify-center rounded-lg bg-muted">
            <Calendar className="size-4 text-muted-foreground" />
          </div>
          <div className="flex min-w-0 flex-col">
            <span className="text-xs text-muted-foreground">Membre depuis</span>
            <span className="truncate text-sm font-medium">{memberSince}</span>
          </div>
        </div>
      </CardContent>
    </Card>
  );
}
