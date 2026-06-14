import type { EnterpriseModuleGrant } from '@nvbes/identity-client';
import { Ban, Mail, UserRoundCheck } from 'lucide-react';
import { useState } from 'react';
import { Alert, AlertDescription, AlertTitle } from '../components/ui/alert';
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
import { Field, FieldError, FieldLabel } from '../components/ui/field';
import { Textarea } from '../components/ui/textarea';
import { describeModuleGrant } from '../enterprise.permissions';
import type { EnterpriseInvitationRow, EnterpriseUserRow } from './UsersPage.table';

type UsersPageInspectorProps = {
  selectedUser: EnterpriseUserRow | null;
  selectedInvitation: EnterpriseInvitationRow | null;
  canEditAccess: boolean;
  mutatingLifecycle: boolean;
  lifecycleError: Error | null;
  onEditAccess: () => void;
  onSuspend: (reason: string) => Promise<boolean>;
  onReactivate: (reason: string) => Promise<boolean>;
};

export function UsersPageInspector({
  selectedUser,
  selectedInvitation,
  canEditAccess,
  mutatingLifecycle,
  lifecycleError,
  onEditAccess,
  onSuspend,
  onReactivate,
}: UsersPageInspectorProps) {
  if (selectedUser) {
    return (
      <MemberInspector
        user={selectedUser}
        canEditAccess={canEditAccess}
        mutatingLifecycle={mutatingLifecycle}
        lifecycleError={lifecycleError}
        onEditAccess={onEditAccess}
        onSuspend={onSuspend}
        onReactivate={onReactivate}
      />
    );
  }

  if (selectedInvitation) {
    return <InvitationInspector invitation={selectedInvitation} />;
  }

  return (
    <Empty className="min-h-72 items-start justify-start border border-dashed bg-muted/20 text-left">
      <div className="flex items-start gap-3">
        <EmptyMedia variant="icon">
          <UserRoundCheck className="size-4" />
        </EmptyMedia>
        <EmptyHeader className="items-start">
          <EmptyTitle>No user selected</EmptyTitle>
          <EmptyDescription>
            Select a member or invitation to inspect access and lifecycle state.
          </EmptyDescription>
        </EmptyHeader>
      </div>
    </Empty>
  );
}

function MemberInspector({
  user,
  canEditAccess,
  mutatingLifecycle,
  lifecycleError,
  onEditAccess,
  onSuspend,
  onReactivate,
}: {
  user: EnterpriseUserRow;
  canEditAccess: boolean;
  mutatingLifecycle: boolean;
  lifecycleError: Error | null;
  onEditAccess: () => void;
  onSuspend: (reason: string) => Promise<boolean>;
  onReactivate: (reason: string) => Promise<boolean>;
}) {
  const [reason, setReason] = useState('');
  const [reasonError, setReasonError] = useState<string | null>(null);
  const suspended = user.status === 'suspended';

  async function submitLifecycle() {
    if (reason.trim().length === 0) {
      setReasonError('An audit reason is required.');
      return;
    }
    setReasonError(null);
    const success = suspended ? await onReactivate(reason) : await onSuspend(reason);
    if (success) {
      setReason('');
    }
  }

  return (
    <Card className="rounded-lg" size="sm">
      <CardHeader>
        <CardTitle>{user.display_name}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        {suspended ? (
          <Alert variant="destructive">
            <Ban className="size-4" />
            <AlertTitle>Suspended member</AlertTitle>
            <AlertDescription>
              This member cannot use tenant resources until reactivated.
            </AlertDescription>
          </Alert>
        ) : null}

        <Detail label="Email" value={user.email} />
        <Detail label="Role" value={user.role} />
        <Detail label="MFA" value={user.mfa_enabled ? 'Enabled' : 'Not enabled'} />
        <Detail label="Workspaces" value={formatWorkspaceCount(user.workspace_ids.length)} />
        <GrantList grants={user.module_grants} />

        {lifecycleError ? (
          <Alert variant="destructive">
            <Ban className="size-4" />
            <AlertTitle>{suspended ? 'Reactivation failed' : 'Suspension failed'}</AlertTitle>
            <AlertDescription>{lifecycleError.message}</AlertDescription>
          </Alert>
        ) : null}

        <div className="flex flex-col gap-2">
          <Button type="button" variant="outline" disabled={!canEditAccess} onClick={onEditAccess}>
            Edit access
          </Button>
          {!canEditAccess ? (
            <p className="text-xs text-muted-foreground">
              Your current tenant grants do not allow member access edits.
            </p>
          ) : null}
        </div>

        <Field>
          <FieldLabel htmlFor="lifecycle-reason">Audit reason</FieldLabel>
          <Textarea
            id="lifecycle-reason"
            value={reason}
            onChange={(event) => setReason(event.target.value)}
            placeholder={
              suspended
                ? 'Why is this member being reactivated?'
                : 'Why is this member being suspended?'
            }
          />
          {reasonError ? <FieldError>{reasonError}</FieldError> : null}
        </Field>
        <Button
          type="button"
          variant={suspended ? 'default' : 'destructive'}
          disabled={!canEditAccess || mutatingLifecycle}
          onClick={submitLifecycle}
        >
          {suspended ? 'Reactivate member' : 'Suspend member'}
        </Button>
      </CardContent>
    </Card>
  );
}

function InvitationInspector({ invitation }: { invitation: EnterpriseInvitationRow }) {
  return (
    <Card className="rounded-lg" size="sm">
      <CardHeader>
        <CardTitle>{invitation.email}</CardTitle>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <Alert>
          <Mail className="size-4" />
          <AlertTitle>Pending invitation</AlertTitle>
          <AlertDescription>This recipient has not joined the tenant yet.</AlertDescription>
        </Alert>
        <Detail label="Role" value={invitation.role} />
        <Detail label="Status" value={invitation.status} />
        <Detail label="Workspaces" value={formatWorkspaceCount(invitation.workspace_ids.length)} />
        <Detail label="Expires" value={formatDate(invitation.expires_at)} />
        <GrantList grants={invitation.module_grants} />
      </CardContent>
    </Card>
  );
}

function Detail({ label, value }: { label: string; value: string }) {
  return (
    <div>
      <p className="text-xs font-medium uppercase text-muted-foreground">{label}</p>
      <p className="mt-1 break-words text-sm">{value}</p>
    </div>
  );
}

function GrantList({ grants }: { grants: EnterpriseModuleGrant[] }) {
  return (
    <div>
      <p className="text-xs font-medium uppercase text-muted-foreground">Module grants</p>
      <div className="mt-2 flex flex-wrap gap-1">
        {grants.length === 0 ? (
          <Badge variant="outline" className="rounded-md">
            None
          </Badge>
        ) : (
          grants.map((grant) => (
            <Badge key={grant} variant="secondary" className="rounded-md">
              {describeModuleGrant(grant)}
            </Badge>
          ))
        )}
      </div>
    </div>
  );
}

function formatWorkspaceCount(count: number): string {
  if (count === 0) {
    return 'No workspaces';
  }
  return count === 1 ? '1 workspace' : `${count} workspaces`;
}

function formatDate(value: string | null | undefined): string {
  if (!value) {
    return 'No expiration returned';
  }
  return new Intl.DateTimeFormat(undefined, { dateStyle: 'medium' }).format(new Date(value));
}
