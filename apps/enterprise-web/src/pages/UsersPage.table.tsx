import type { EnterpriseUsersResponse } from '@nvbes/identity-client';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '../components/ui/table';
import { describeModuleGrant } from '../enterprise.permissions';

export type EnterpriseUserRow = EnterpriseUsersResponse['users'][number];
export type EnterpriseInvitationRow = EnterpriseUsersResponse['invitations'][number];

export type UsersPageSelection =
  | { type: 'member'; id: string }
  | { type: 'invitation'; id: string };

type DirectoryRow =
  | { type: 'member'; id: string; user: EnterpriseUserRow }
  | { type: 'invitation'; id: string; invitation: EnterpriseInvitationRow };

type UsersPageTableProps = {
  users: EnterpriseUserRow[];
  invitations: EnterpriseInvitationRow[];
  selected: UsersPageSelection | null;
  onSelect: (selection: UsersPageSelection) => void;
};

export function UsersPageTable({ users, invitations, selected, onSelect }: UsersPageTableProps) {
  const rows: DirectoryRow[] = [
    ...users.map((user) => ({ type: 'member' as const, id: user.id, user })),
    ...invitations.map((invitation) => ({
      type: 'invitation' as const,
      id: invitation.id,
      invitation,
    })),
  ];

  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>Name / email</TableHead>
          <TableHead>Role</TableHead>
          <TableHead>Module grants</TableHead>
          <TableHead>Workspaces</TableHead>
          <TableHead>Status</TableHead>
          <TableHead>Last active</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((row) => {
          const subject = row.type === 'member' ? row.user : row.invitation;
          const rowSelection = { type: row.type, id: row.id };
          const isSelected = selected?.type === row.type && selected.id === row.id;

          return (
            <TableRow
              key={`${row.type}:${row.id}`}
              data-state={isSelected ? 'selected' : undefined}
            >
              <TableCell className="min-w-52">
                <Button
                  type="button"
                  variant="link"
                  className="h-auto min-w-0 justify-start p-0 text-left"
                  onClick={() => onSelect(rowSelection)}
                >
                  <span className="min-w-0">
                    <span className="block truncate font-medium">
                      {row.type === 'member' ? row.user.display_name : row.invitation.email}
                    </span>
                    <span className="block truncate text-xs font-normal text-muted-foreground">
                      {subject.email}
                    </span>
                  </span>
                </Button>
              </TableCell>
              <TableCell>
                <Badge variant="outline" className="rounded-md capitalize">
                  {subject.role}
                </Badge>
              </TableCell>
              <TableCell className="max-w-64">
                <GrantList grants={subject.module_grants} />
              </TableCell>
              <TableCell>{formatWorkspaceCount(subject.workspace_ids.length)}</TableCell>
              <TableCell>
                <Badge
                  variant={subject.status === 'active' ? 'secondary' : 'outline'}
                  className="rounded-md capitalize"
                >
                  {subject.status}
                </Badge>
              </TableCell>
              <TableCell className="text-muted-foreground">
                {row.type === 'member' ? formatDate(row.user.last_seen_at) : 'Invitation pending'}
              </TableCell>
            </TableRow>
          );
        })}
      </TableBody>
    </Table>
  );
}

function GrantList({ grants }: { grants: EnterpriseUserRow['module_grants'] }) {
  if (grants.length === 0) {
    return <span className="text-muted-foreground">No module grants</span>;
  }

  return (
    <div className="flex flex-wrap gap-1">
      {grants.slice(0, 3).map((grant) => (
        <Badge key={grant} variant="secondary" className="rounded-md">
          {describeModuleGrant(grant)}
        </Badge>
      ))}
      {grants.length > 3 ? (
        <Badge variant="outline" className="rounded-md">
          +{grants.length - 3}
        </Badge>
      ) : null}
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
    return 'Never';
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: 'medium',
    timeStyle: 'short',
  }).format(new Date(value));
}
