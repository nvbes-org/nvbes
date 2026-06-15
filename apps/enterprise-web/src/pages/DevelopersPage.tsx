import type { EnterpriseDevelopersResponse } from '@nvbes/identity-client';
import { useQuery } from '@tanstack/react-query';
import { AlertCircle, Code2, KeyRound, RotateCcw, TimerReset } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '../components/ui/alert';
import { Badge } from '../components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '../components/ui/empty';
import { Skeleton } from '../components/ui/skeleton';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table';
import { enterpriseDevelopersQueryOptions } from '../enterprise.queries';

type DeveloperCredential = EnterpriseDevelopersResponse['credentials'][number];

const rotationWindowDays = 90;

export function DevelopersPage() {
  const developersQuery = useQuery(enterpriseDevelopersQueryOptions());
  const credentials = developersQuery.data?.credentials ?? [];
  const rotationCandidates = credentials.filter(needsRotation);

  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-2xl font-heading font-semibold">Developers</h1>
            <Badge variant="outline" className="rounded-md">
              Secrets
            </Badge>
          </div>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
            OAuth clients and developer credentials that affect tenant integration security.
          </p>
        </div>
        <Badge
          variant={rotationCandidates.length > 0 ? 'secondary' : 'default'}
          className="rounded-md"
        >
          {rotationCandidates.length} rotation candidates
        </Badge>
      </header>

      {developersQuery.error ? <DevelopersErrorAlert error={developersQuery.error} /> : null}

      {developersQuery.isPending ? (
        <DevelopersLoadingState />
      ) : (
        <>
          <section className="grid gap-3 md:grid-cols-3">
            <DeveloperMetric
              icon={Code2}
              label="Credentials"
              value={`${credentials.length}`}
              detail="Active developer entries"
            />
            <DeveloperMetric
              icon={RotateCcw}
              label="Rotation candidates"
              value={`${rotationCandidates.length}`}
              detail={`${rotationWindowDays}+ days or expired`}
            />
            <DeveloperMetric
              icon={KeyRound}
              label="Scoped credentials"
              value={`${credentials.filter((credential) => credential.scopes.length > 0).length}`}
              detail="Credentials with explicit scopes"
            />
          </section>

          <CredentialsCard credentials={credentials} />
        </>
      )}
    </div>
  );
}

function DeveloperMetric({
  icon: Icon,
  label,
  value,
  detail,
}: {
  icon: typeof Code2;
  label: string;
  value: string;
  detail: string;
}) {
  return (
    <Card className="min-h-28 rounded-lg" size="sm">
      <CardContent>
        <div className="flex items-center justify-between gap-3">
          <p className="truncate text-xs font-medium text-muted-foreground">{label}</p>
          <Icon className="size-4 shrink-0 text-muted-foreground" />
        </div>
        <p className="mt-4 truncate text-xl font-heading font-semibold">{value}</p>
        <p className="mt-1 truncate text-xs text-muted-foreground">{detail}</p>
      </CardContent>
    </Card>
  );
}

function CredentialsCard({ credentials }: { credentials: DeveloperCredential[] }) {
  return (
    <Card className="rounded-lg" size="sm">
      <CardHeader className="border-b border-border">
        <CardTitle>Credential inventory</CardTitle>
      </CardHeader>
      <CardContent>
        {credentials.length > 0 ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Scopes</TableHead>
                <TableHead>Last used</TableHead>
                <TableHead>Expires</TableHead>
                <TableHead>Rotation</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {credentials.map((credential) => (
                <TableRow key={credential.id}>
                  <TableCell className="font-medium">{credential.name}</TableCell>
                  <TableCell>{credential.scopes.length || 'None'}</TableCell>
                  <TableCell>{formatDate(credential.last_used_at)}</TableCell>
                  <TableCell>{formatDate(credential.expires_at)}</TableCell>
                  <TableCell>
                    <RotationBadge credential={credential} />
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <Empty className="min-h-56 border bg-muted/20">
            <EmptyMedia variant="icon">
              <TimerReset className="size-4" />
            </EmptyMedia>
            <EmptyHeader>
              <EmptyTitle>No developer credentials</EmptyTitle>
              <EmptyDescription>
                OAuth clients and service account credentials will appear here when configured.
              </EmptyDescription>
            </EmptyHeader>
          </Empty>
        )}
      </CardContent>
    </Card>
  );
}

function RotationBadge({ credential }: { credential: DeveloperCredential }) {
  if (isExpired(credential.expires_at)) {
    return (
      <Badge variant="destructive" className="rounded-md">
        Expired
      </Badge>
    );
  }

  if (needsRotation(credential)) {
    return (
      <Badge variant="secondary" className="rounded-md">
        Review
      </Badge>
    );
  }

  return (
    <Badge variant="outline" className="rounded-md">
      Current
    </Badge>
  );
}

function DevelopersLoadingState() {
  return (
    <div className="flex flex-col gap-4">
      <div className="grid gap-3 md:grid-cols-3">
        <Skeleton className="h-28 w-full rounded-lg" />
        <Skeleton className="h-28 w-full rounded-lg" />
        <Skeleton className="h-28 w-full rounded-lg" />
      </div>
      <Skeleton className="h-72 w-full rounded-lg" />
    </div>
  );
}

function DevelopersErrorAlert({ error }: { error: unknown }) {
  return (
    <Alert variant="destructive">
      <AlertCircle className="size-4" />
      <AlertTitle>Developer credentials unavailable</AlertTitle>
      <AlertDescription>
        {error instanceof Error ? error.message : 'Unable to load developer credentials.'}
      </AlertDescription>
    </Alert>
  );
}

function needsRotation(credential: DeveloperCredential): boolean {
  if (isExpired(credential.expires_at)) {
    return true;
  }

  const createdAt = Date.parse(credential.created_at);
  if (Number.isNaN(createdAt)) {
    return false;
  }

  const ageDays = (Date.now() - createdAt) / 86_400_000;
  return ageDays >= rotationWindowDays;
}

function isExpired(value: string | null | undefined): boolean {
  return Boolean(value && Date.parse(value) <= Date.now());
}

function formatDate(value: string | null | undefined): string {
  if (!value) {
    return 'Never';
  }

  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return 'Invalid date';
  }

  return new Intl.DateTimeFormat(undefined, {
    day: '2-digit',
    month: 'short',
    year: 'numeric',
  }).format(date);
}
