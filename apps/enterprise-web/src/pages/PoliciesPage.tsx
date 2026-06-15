import type {
  EnterprisePoliciesResponse,
  EnterprisePolicySimulationInput,
} from '@nvbes/identity-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertCircle, Clock3, Save, ShieldCheck } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Alert, AlertDescription, AlertTitle } from '../components/ui/alert';
import { Badge } from '../components/ui/badge';
import { Button } from '../components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
import { Input } from '../components/ui/input';
import { Skeleton } from '../components/ui/skeleton';
import { enterpriseClient } from '../enterprise.api';
import { canManagePolicies } from '../enterprise.permissions';
import {
  enterpriseContextQueryOptions,
  enterpriseQueryKeys,
  enterprisePoliciesQueryOptions,
  enterpriseUsersQueryOptions,
  enterpriseWorkspacesQueryOptions,
} from '../enterprise.queries';
import { PoliciesPageForm, type PoliciesPageSubjectType } from './PoliciesPage.form';
import { SimulationResult } from './PoliciesPage.result';

export function PoliciesPage() {
  const queryClient = useQueryClient();
  const contextQuery = useQuery(enterpriseContextQueryOptions());
  const policiesQuery = useQuery(enterprisePoliciesQueryOptions());
  const usersQuery = useQuery(enterpriseUsersQueryOptions());
  const workspacesQuery = useQuery(enterpriseWorkspacesQueryOptions());
  const [workspaceId, setWorkspaceId] = useState('');
  const [subjectType, setSubjectType] = useState<PoliciesPageSubjectType>('user');
  const [userId, setUserId] = useState('');
  const [clientId, setClientId] = useState('');
  const [action, setAction] = useState('view_files');
  const [ownsResource, setOwnsResource] = useState(false);
  const [memberShareLinksEnabled, setMemberShareLinksEnabled] = useState(false);
  const [targetRole, setTargetRole] = useState('none');

  const workspaces = workspacesQuery.data?.workspaces ?? [];
  const users = usersQuery.data?.users ?? [];
  const activePolicies =
    policiesQuery.data?.policies.filter((policy) => policy.enabled).length ?? 0;
  const sessionPolicy = policiesQuery.data?.session_policy;
  const canSimulate = contextQuery.data ? canManagePolicies(contextQuery.data) : false;
  const selectedUser = users.find((user) => user.id === userId);
  const selectedWorkspace = workspaces.find((workspace) => workspace.id === workspaceId);

  useEffect(() => {
    if (!workspaceId && workspaces[0]) {
      setWorkspaceId(workspaces[0].id);
    }
    if (!userId && users[0]) {
      setUserId(users[0].id);
    }
  }, [workspaceId, workspaces, userId, users]);

  const simulationMutation = useMutation({
    mutationFn: (input: EnterprisePolicySimulationInput) =>
      enterpriseClient.simulateEnterprisePolicy(input),
  });
  const sessionPolicyMutation = useMutation({
    mutationFn: (adminSessionTtlHours: number) =>
      enterpriseClient.updateEnterpriseSessionPolicy({
        admin_session_ttl_hours: adminSessionTtlHours,
      }),
    onSuccess: (data) => {
      queryClient.setQueryData(enterpriseQueryKeys.policies, data);
    },
  });

  const isLoading =
    contextQuery.isPending ||
    policiesQuery.isPending ||
    usersQuery.isPending ||
    workspacesQuery.isPending;
  const error =
    contextQuery.error ?? policiesQuery.error ?? usersQuery.error ?? workspacesQuery.error ?? null;
  const subjectReady = subjectType === 'user' ? Boolean(userId) : clientId.trim().length > 0;
  const submitDisabled =
    !canSimulate || !workspaceId || !subjectReady || simulationMutation.isPending || isLoading;

  return (
    <div className="flex flex-col gap-6">
      <header className="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
        <div>
          <div className="flex items-center gap-2">
            <h1 className="text-2xl font-heading font-semibold">Policies</h1>
            <Badge variant="outline" className="rounded-md">
              Simulator
            </Badge>
          </div>
          <p className="mt-2 max-w-2xl text-sm leading-6 text-muted-foreground">
            Test effective workspace authorization for a selected user or service client before
            changing policy configuration.
          </p>
        </div>
        <PolicyStats activePolicies={activePolicies} workspaceCount={workspaces.length} />
      </header>

      <QueryErrorAlert error={error} />
      <PolicyGrantAlert visible={!canSimulate && !contextQuery.isPending} />

      {isLoading ? (
        <LoadingState />
      ) : (
        <div className="flex flex-col gap-4">
          {sessionPolicy ? (
            <SessionPolicyCard
              error={sessionPolicyMutation.error}
              pending={sessionPolicyMutation.isPending}
              sessionPolicy={sessionPolicy}
              onSave={(value) => sessionPolicyMutation.mutate(value)}
            />
          ) : null}

          <section className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_340px]">
            <Card className="rounded-lg" size="sm">
              <CardHeader>
                <CardTitle>Policy simulator</CardTitle>
              </CardHeader>
              <CardContent>
                <PoliciesPageForm
                  action={action}
                  clientId={clientId}
                  disabled={submitDisabled}
                  memberShareLinksEnabled={memberShareLinksEnabled}
                  ownsResource={ownsResource}
                  selectedUserRole={selectedUser?.role ?? null}
                  selectedWorkspaceId={selectedWorkspace?.id ?? null}
                  subjectType={subjectType}
                  targetRole={targetRole}
                  userId={userId}
                  users={users}
                  workspaceId={workspaceId}
                  workspaces={workspaces}
                  onActionChange={setAction}
                  onClientIdChange={setClientId}
                  onMemberShareLinksEnabledChange={setMemberShareLinksEnabled}
                  onOwnsResourceChange={setOwnsResource}
                  onSubmit={simulationMutation.mutate}
                  onSubjectTypeChange={setSubjectType}
                  onTargetRoleChange={setTargetRole}
                  onUserIdChange={setUserId}
                  onWorkspaceIdChange={setWorkspaceId}
                />
              </CardContent>
            </Card>

            <SimulationResult
              result={simulationMutation.data ?? null}
              error={simulationMutation.error}
              pending={simulationMutation.isPending}
            />
          </section>
        </div>
      )}
    </div>
  );
}

type SessionPolicyCardProps = {
  error: Error | null;
  pending: boolean;
  onSave(value: number): void;
  sessionPolicy: EnterprisePoliciesResponse['session_policy'];
};

function SessionPolicyCard(props: SessionPolicyCardProps) {
  const { error, pending, sessionPolicy } = props;
  const [value, setValue] = useState(String(sessionPolicy.admin_session_ttl_hours));
  const parsedValue = Number.parseInt(value, 10);
  const validValue = Number.isInteger(parsedValue) && parsedValue >= 1 && parsedValue <= 168;
  const dirty = parsedValue !== sessionPolicy.admin_session_ttl_hours;

  useEffect(() => {
    setValue(String(sessionPolicy.admin_session_ttl_hours));
  }, [sessionPolicy.admin_session_ttl_hours]);

  return (
    <Card className="rounded-lg" size="sm">
      <CardContent className="flex flex-col gap-4 md:flex-row md:items-start md:justify-between">
        <div className="flex min-w-0 items-start gap-3">
          <div className="rounded-lg border border-border bg-muted/30 p-2 text-muted-foreground">
            <Clock3 className="size-4" />
          </div>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <p className="text-sm font-semibold">Admin session TTL</p>
              <Badge
                variant={sessionPolicy.compliant ? 'default' : 'secondary'}
                className="rounded-md"
              >
                {sessionPolicy.compliant ? 'Compliant' : 'Review'}
              </Badge>
            </div>
            <p className="mt-1 text-sm leading-5 text-muted-foreground">
              Current admin session TTL is {sessionPolicy.admin_session_ttl_hours}h. Recommended
              maximum is {sessionPolicy.recommended_admin_session_ttl_hours}h with step-up for admin
              elevation. Source: {sessionPolicy.source}.
            </p>
            {error ? <p className="mt-2 text-sm text-destructive">{error.message}</p> : null}
          </div>
        </div>
        <form
          className="grid gap-2 md:min-w-64"
          onSubmit={(event) => {
            event.preventDefault();
            if (validValue) {
              props.onSave(parsedValue);
            }
          }}
        >
          <label className="text-xs font-medium text-muted-foreground" htmlFor="admin-session-ttl">
            Admin session TTL hours
          </label>
          <div className="flex gap-2">
            <Input
              id="admin-session-ttl"
              min={1}
              max={168}
              type="number"
              value={value}
              onChange={(event) => setValue(event.target.value)}
            />
            <Button type="submit" disabled={!dirty || !validValue || pending}>
              <Save className="size-4" />
              Save
            </Button>
          </div>
          <p className="text-xs text-muted-foreground">Allowed range: 1-168 hours.</p>
        </form>
      </CardContent>
    </Card>
  );
}

function PolicyStats({
  activePolicies,
  workspaceCount,
}: {
  activePolicies: number;
  workspaceCount: number;
}) {
  return (
    <div className="grid grid-cols-2 gap-2 text-right text-sm">
      <span className="text-muted-foreground">Active policies</span>
      <strong>{activePolicies}</strong>
      <span className="text-muted-foreground">Workspaces</span>
      <strong>{workspaceCount}</strong>
    </div>
  );
}

function PolicyGrantAlert({ visible }: { visible: boolean }) {
  if (!visible) {
    return null;
  }
  return (
    <Alert>
      <ShieldCheck className="size-4" />
      <AlertTitle>Policies grant required</AlertTitle>
      <AlertDescription>
        Your current tenant grants do not allow policy simulation.
      </AlertDescription>
    </Alert>
  );
}

function LoadingState() {
  return (
    <section className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_340px]">
      <Card className="rounded-lg" size="sm">
        <CardContent className="grid gap-3 md:grid-cols-2">
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
          <Skeleton className="h-12 w-full" />
        </CardContent>
      </Card>
      <Skeleton className="h-72 w-full rounded-lg" />
    </section>
  );
}

function QueryErrorAlert({ error }: { error: Error | null }) {
  if (!error) {
    return null;
  }
  return (
    <Alert variant="destructive">
      <AlertCircle className="size-4" />
      <AlertTitle>Policies data failed to load</AlertTitle>
      <AlertDescription>{error.message}</AlertDescription>
    </Alert>
  );
}
