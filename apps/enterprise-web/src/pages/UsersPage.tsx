import type { EnterpriseModuleGrant, EnterpriseRole } from '@nvbes/identity-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertCircle, UsersRound } from 'lucide-react';
import { useMemo, useState } from 'react';
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
import { enterpriseClient } from '../enterprise.api';
import { buildEnterpriseInvitationInput } from '../enterprise.invites';
import { canManageUsers } from '../enterprise.permissions';
import {
  enterpriseContextQueryOptions,
  enterpriseQueryKeys,
  enterpriseUsersQueryOptions,
} from '../enterprise.queries';
import { type AccessFormValue, UsersPageAccess } from './UsersPage.access';
import { UsersPageInspector } from './UsersPage.inspector';
import { type InviteSubmitValue, UsersPageInvite } from './UsersPage.invite';
import {
  type EnterpriseInvitationRow,
  type EnterpriseUserRow,
  UsersPageTable,
  type UsersPageSelection,
} from './UsersPage.table';

type RecipientError = { email: string; message: string };

export function UsersPage() {
  const queryClient = useQueryClient();
  const contextQuery = useQuery(enterpriseContextQueryOptions());
  const usersQuery = useQuery(enterpriseUsersQueryOptions());
  const [selected, setSelected] = useState<UsersPageSelection | null>(null);
  const [accessOpen, setAccessOpen] = useState(false);
  const [recipientErrors, setRecipientErrors] = useState<RecipientError[]>([]);

  const usersData = usersQuery.data;
  const selectedUser = findSelectedUser(usersData?.users ?? [], selected);
  const selectedInvitation = findSelectedInvitation(usersData?.invitations ?? [], selected);
  const workspaceIds = useMemo(
    () => collectWorkspaceIds(usersData?.users ?? [], usersData?.invitations ?? []),
    [usersData],
  );
  const canEditAccess = contextQuery.data ? canManageUsers(contextQuery.data) : false;

  const invalidateUsers = () =>
    queryClient.invalidateQueries({ queryKey: enterpriseQueryKeys.users });
  const inviteMutation = useMutation({
    mutationFn: (value: InviteSubmitValue) =>
      enterpriseClient.createEnterpriseInvitations(buildEnterpriseInvitationInput(value)),
    onSuccess: async () => {
      setRecipientErrors([]);
      await invalidateUsers();
    },
  });
  const accessMutation = useMutation({
    mutationFn: (value: AccessFormValue) => {
      if (!selectedUser) {
        throw new Error('No member selected.');
      }
      return enterpriseClient.updateEnterpriseUserAccess(selectedUser.id, value);
    },
    onSuccess: async () => {
      setAccessOpen(false);
      await invalidateUsers();
    },
  });
  const suspendMutation = useMutation({
    mutationFn: ({ userId, reason }: { userId: string; reason: string }) =>
      enterpriseClient.suspendEnterpriseUser(userId, { reason }),
    onSuccess: invalidateUsers,
  });
  const reactivateMutation = useMutation({
    mutationFn: ({ userId, reason }: { userId: string; reason: string }) =>
      enterpriseClient.reactivateEnterpriseUser(userId, { reason }),
    onSuccess: invalidateUsers,
  });

  const roles = usersData?.roles ?? fallbackRoles;
  const grants = usersData?.module_grants ?? fallbackGrants;
  const isLoading = usersQuery.isPending || contextQuery.isPending;
  const isEmpty = usersData && usersData.users.length === 0 && usersData.invitations.length === 0;

  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-2xl font-heading font-semibold">Users</h1>
            <Badge variant="outline" className="rounded-md">
              V0
            </Badge>
          </div>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
            Tenant member directory, invitations, and access lifecycle controls.
          </p>
        </div>
        <UsersPageInvite
          roles={roles}
          grants={grants}
          workspaceIds={workspaceIds}
          submitting={inviteMutation.isPending}
          recipientErrors={recipientErrors}
          onSubmit={async (value) => {
            try {
              await inviteMutation.mutateAsync(value);
              return true;
            } catch (error) {
              setRecipientErrors(buildRecipientErrors(value.emailInput, error));
              return false;
            }
          }}
        />
      </header>

      <QueryErrorAlert error={usersQuery.error ?? contextQuery.error} />

      {isLoading ? (
        <LoadingState />
      ) : isEmpty ? (
        <EmptyState />
      ) : usersData ? (
        <section className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
          <Card className="rounded-lg" size="sm">
            <CardHeader>
              <CardTitle>Directory</CardTitle>
            </CardHeader>
            <CardContent>
              <UsersPageTable
                users={usersData.users}
                invitations={usersData.invitations}
                selected={selected}
                onSelect={setSelected}
              />
            </CardContent>
          </Card>

          <UsersPageInspector
            selectedUser={selectedUser}
            selectedInvitation={selectedInvitation}
            canEditAccess={canEditAccess}
            mutatingLifecycle={suspendMutation.isPending || reactivateMutation.isPending}
            onEditAccess={() => setAccessOpen(true)}
            onSuspend={(reason) => {
              if (selectedUser) {
                suspendMutation.mutate({ userId: selectedUser.id, reason });
              }
            }}
            onReactivate={(reason) => {
              if (selectedUser) {
                reactivateMutation.mutate({ userId: selectedUser.id, reason });
              }
            }}
          />
        </section>
      ) : null}

      {selectedUser ? (
        <UsersPageAccess
          open={accessOpen}
          title={selectedUser.email}
          roles={roles}
          grants={grants}
          workspaceIds={workspaceIds}
          value={{
            role: selectedUser.role,
            module_grants: selectedUser.module_grants,
            workspace_ids: selectedUser.workspace_ids,
          }}
          saving={accessMutation.isPending}
          onOpenChange={setAccessOpen}
          onSubmit={(value) => accessMutation.mutate(value)}
        />
      ) : null}
    </div>
  );
}

function LoadingState() {
  return (
    <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_320px]">
      <Card className="rounded-lg" size="sm">
        <CardContent className="space-y-3">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </CardContent>
      </Card>
      <Skeleton className="h-72 w-full rounded-lg" />
    </div>
  );
}

function EmptyState() {
  return (
    <Empty className="min-h-72 border bg-muted/20">
      <EmptyMedia variant="icon">
        <UsersRound className="size-4" />
      </EmptyMedia>
      <EmptyHeader>
        <EmptyTitle>No users or invitations</EmptyTitle>
        <EmptyDescription>
          Invite the first tenant admins to start managing enterprise access.
        </EmptyDescription>
      </EmptyHeader>
    </Empty>
  );
}

function QueryErrorAlert({ error }: { error: Error | null }) {
  if (!error) {
    return null;
  }
  return (
    <Alert variant="destructive">
      <AlertCircle className="size-4" />
      <AlertTitle>Users data failed to load</AlertTitle>
      <AlertDescription>{error.message}</AlertDescription>
    </Alert>
  );
}

function findSelectedUser(users: EnterpriseUserRow[], selected: UsersPageSelection | null) {
  return selected?.type === 'member'
    ? (users.find((user) => user.id === selected.id) ?? null)
    : null;
}

function findSelectedInvitation(
  invitations: EnterpriseInvitationRow[],
  selected: UsersPageSelection | null,
) {
  return selected?.type === 'invitation'
    ? (invitations.find((invitation) => invitation.id === selected.id) ?? null)
    : null;
}

function collectWorkspaceIds(
  users: EnterpriseUserRow[],
  invitations: EnterpriseInvitationRow[],
): string[] {
  return Array.from(
    new Set([...users, ...invitations].flatMap((subject) => subject.workspace_ids)),
  ).sort();
}

function buildRecipientErrors(emailInput: string, error: unknown): RecipientError[] {
  const message = error instanceof Error ? error.message : 'Invitation failed.';
  const emails = emailInput
    .split(/[,\n]/)
    .map((email) => email.trim())
    .filter((email) => email.length > 0);
  return emails.length > 0
    ? emails.map((email) => ({ email, message }))
    : [{ email: 'recipient', message }];
}

const fallbackRoles: EnterpriseRole[] = ['owner', 'admin', 'member', 'viewer'];
const fallbackGrants: EnterpriseModuleGrant[] = [
  'members',
  'workspaces',
  'developers',
  'policies',
  'security',
  'billing',
  'audit',
  'drive',
];
