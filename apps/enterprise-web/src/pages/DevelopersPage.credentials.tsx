import type { EnterpriseDevelopersResponse } from '@nvbes/identity-client';
import { TimerReset, Trash2 } from 'lucide-react';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from '../components/ui/empty';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table';

type DeveloperCredential = EnterpriseDevelopersResponse['credentials'][number];

export const rotationWindowDays = 90;

export function CredentialsCard({
  credentials,
  pendingCredentialId,
  revokeError,
  pending,
  onRevoke,
}: {
  credentials: DeveloperCredential[];
  pendingCredentialId: string | null;
  revokeError: Error | null;
  pending: boolean;
  onRevoke: (credentialId: string) => Promise<void>;
}) {
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
                <TableHead>Client</TableHead>
                <TableHead>Secret</TableHead>
                <TableHead>Scopes</TableHead>
                <TableHead>Last used</TableHead>
                <TableHead>Expires</TableHead>
                <TableHead>Rotation</TableHead>
                <TableHead className="w-24">Action</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {credentials.map((credential) => (
                <TableRow key={credential.id}>
                  <TableCell className="font-medium">{credential.name}</TableCell>
                  <TableCell>{credential.client_id}</TableCell>
                  <TableCell>
                    <Badge variant="outline" className="rounded-md">
                      {credential.status} / {credential.secret_last4}
                    </Badge>
                  </TableCell>
                  <TableCell>{credential.scopes.length || 'None'}</TableCell>
                  <TableCell>{formatDate(credential.last_used_at)}</TableCell>
                  <TableCell>{formatDate(credential.expires_at)}</TableCell>
                  <TableCell>
                    <RotationBadge credential={credential} />
                  </TableCell>
                  <TableCell>
                    <Button
                      size="sm"
                      variant="outline"
                      disabled={!canRevoke(credential) || pending}
                      onClick={() => void onRevoke(credential.id)}
                    >
                      <Trash2 className="size-4" />
                      {pendingCredentialId === credential.id ? 'Revoking...' : 'Revoke'}
                    </Button>
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
        {revokeError ? (
          <p className="mt-3 text-sm text-destructive">{revokeError.message}</p>
        ) : null}
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

export function needsRotation(credential: DeveloperCredential): boolean {
  if (credential.status === 'overlap') {
    return true;
  }

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

function canRevoke(credential: DeveloperCredential): boolean {
  return credential.status !== 'client' && needsRotation(credential);
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
