import type { EnterprisePolicySimulationInput } from '@nvbes/identity-client';
import { Button } from '../components/ui/button';
import { Checkbox } from '../components/ui/checkbox';
import { Field, FieldDescription, FieldLabel } from '../components/ui/field';
import { Input } from '../components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../components/ui/select';

export type PoliciesPageSubjectType = 'user' | 'client';

type PoliciesPageFormProps = {
  action: string;
  clientId: string;
  disabled: boolean;
  memberShareLinksEnabled: boolean;
  ownsResource: boolean;
  selectedUserRole: string | null;
  selectedWorkspaceId: string | null;
  subjectType: PoliciesPageSubjectType;
  targetRole: string;
  userId: string;
  users: Array<{ id: string; email: string; role: string }>;
  workspaceId: string;
  workspaces: Array<{ id: string; name: string }>;
  onActionChange(value: string): void;
  onClientIdChange(value: string): void;
  onMemberShareLinksEnabledChange(value: boolean): void;
  onOwnsResourceChange(value: boolean): void;
  onSubmit(input: EnterprisePolicySimulationInput): void;
  onSubjectTypeChange(value: PoliciesPageSubjectType): void;
  onTargetRoleChange(value: string): void;
  onUserIdChange(value: string): void;
  onWorkspaceIdChange(value: string): void;
};

const actions = [
  { value: 'view_files', label: 'View files' },
  { value: 'upload_file', label: 'Upload file' },
  { value: 'create_share_link', label: 'Create share link' },
  { value: 'change_member_role', label: 'Change member role' },
  { value: 'manage_billing', label: 'Manage billing' },
  { value: 'delete_workspace', label: 'Delete workspace' },
];

const targetRoles = ['owner', 'admin', 'security_admin', 'billing_admin', 'member', 'viewer'];

export function PoliciesPageForm(props: PoliciesPageFormProps) {
  return (
    <form
      className="grid gap-4 md:grid-cols-2"
      onSubmit={(event) => {
        event.preventDefault();
        props.onSubmit(buildSimulationInput(props));
      }}
    >
      <Field>
        <FieldLabel>Workspace</FieldLabel>
        <Select value={props.workspaceId} onValueChange={props.onWorkspaceIdChange}>
          <SelectTrigger className="w-full">
            <SelectValue placeholder="Select workspace" />
          </SelectTrigger>
          <SelectContent>
            {props.workspaces.map((workspace) => (
              <SelectItem key={workspace.id} value={workspace.id}>
                {workspace.name}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <FieldDescription>{props.selectedWorkspaceId ?? 'No workspace selected'}</FieldDescription>
      </Field>

      <Field>
        <FieldLabel>Subject type</FieldLabel>
        <Select
          value={props.subjectType}
          onValueChange={(value) => props.onSubjectTypeChange(value as PoliciesPageSubjectType)}
        >
          <SelectTrigger className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="user">User</SelectItem>
            <SelectItem value="client">Client</SelectItem>
          </SelectContent>
        </Select>
      </Field>

      {props.subjectType === 'user' ? (
        <Field>
          <FieldLabel>User</FieldLabel>
          <Select value={props.userId} onValueChange={props.onUserIdChange}>
            <SelectTrigger className="w-full">
              <SelectValue placeholder="Select user" />
            </SelectTrigger>
            <SelectContent>
              {props.users.map((user) => (
                <SelectItem key={user.id} value={user.id}>
                  {user.email}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <FieldDescription>{props.selectedUserRole ?? 'No user selected'}</FieldDescription>
        </Field>
      ) : (
        <Field>
          <FieldLabel>Client ID</FieldLabel>
          <Input
            value={props.clientId}
            onChange={(event) => props.onClientIdChange(event.target.value)}
            placeholder="service-client-id"
          />
        </Field>
      )}

      <Field>
        <FieldLabel>Action</FieldLabel>
        <Select value={props.action} onValueChange={props.onActionChange}>
          <SelectTrigger className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {actions.map((item) => (
              <SelectItem key={item.value} value={item.value}>
                {item.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>

      <Field>
        <FieldLabel>Target role</FieldLabel>
        <Select value={props.targetRole} onValueChange={props.onTargetRoleChange}>
          <SelectTrigger className="w-full">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="none">None</SelectItem>
            {targetRoles.map((role) => (
              <SelectItem key={role} value={role}>
                {role}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </Field>

      <div className="flex flex-col justify-end gap-3">
        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={props.ownsResource}
            onCheckedChange={(checked) => props.onOwnsResourceChange(checked === true)}
          />
          Owns target resource
        </label>
        <label className="flex items-center gap-2 text-sm">
          <Checkbox
            checked={props.memberShareLinksEnabled}
            onCheckedChange={(checked) => props.onMemberShareLinksEnabledChange(checked === true)}
          />
          Force member share links
        </label>
      </div>

      <div className="md:col-span-2">
        <Button type="submit" disabled={props.disabled}>
          Run simulation
        </Button>
      </div>
    </form>
  );
}

function buildSimulationInput(props: PoliciesPageFormProps): EnterprisePolicySimulationInput {
  return {
    workspace_id: props.workspaceId,
    subject:
      props.subjectType === 'user'
        ? { subject_type: 'user', user_id: props.userId }
        : { subject_type: 'client', client_id: props.clientId.trim() },
    action: props.action,
    resource: {
      owns_resource: props.ownsResource,
      member_share_links_enabled: props.memberShareLinksEnabled,
      target_role: props.targetRole === 'none' ? undefined : props.targetRole,
    },
  };
}
