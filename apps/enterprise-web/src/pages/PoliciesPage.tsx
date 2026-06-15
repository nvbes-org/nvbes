import type {
  EnterprisePoliciesResponse,
  EnterprisePolicySimulationInput,
} from '@nvbes/identity-client';
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { AlertCircle, ShieldCheck } from 'lucide-react';
import { useEffect, useState } from 'react';
import { Alert, AlertDescription, AlertTitle } from '../components/ui/alert';
import { Badge } from '../components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '../components/ui/card';
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
import { MfaPolicyCard, SessionPolicyCard } from './PoliciesPage.security';
import { isAdminElevationCancelled, useAdminElevation } from './UsersPage.admin-elevation';

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
  const adminElevation = useAdminElevation({
    active: contextQuery.data?.admin_elevation.active ?? false,
    breakGlass: contextQuery.data?.break_glass ?? null,
    onGranted: () => queryClient.invalidateQueries({ queryKey: enterpriseQueryKeys.context }),
  });
  const adminWithoutMfaCount = users.filter(
    (user) => (user.role === 'owner' || user.role === 'admin') && !user.mfa_enabled,
  ).length;

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
      adminElevation.runElevated(() =>
        enterpriseClient.updateEnterpriseSessionPolicy({
          admin_session_ttl_hours: adminSessionTtlHours,
        }),
      ),
    onSuccess: (data) => {
      queryClient.setQueryData(enterpriseQueryKeys.policies, data);
    },
  });
  const mfaPolicyMutation = useMutation({
    mutationFn: (policy: EnterprisePoliciesResponse['mfa_policy']['policy']) =>
      adminElevation.runElevated(() => enterpriseClient.updateEnterpriseMfaPolicy({ policy })),
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
              pending={sessionPolicyMutation.isPending || adminElevation.pending}
              sessionPolicy={sessionPolicy}
              onSave={async (value) => {
                try {
                  await sessionPolicyMutation.mutateAsync(value);
                } catch (error) {
                  if (isAdminElevationCancelled(error)) {
                    sessionPolicyMutation.reset();
                  }
                }
              }}
            />
          ) : null}
          {policiesQuery.data?.mfa_policy ? (
            <MfaPolicyCard
              adminWithoutMfaCount={adminWithoutMfaCount}
              error={mfaPolicyMutation.error}
              mfaPolicy={policiesQuery.data.mfa_policy}
              pending={mfaPolicyMutation.isPending || adminElevation.pending}
              onSave={async (value) => {
                try {
                  await mfaPolicyMutation.mutateAsync(value);
                } catch (error) {
                  if (isAdminElevationCancelled(error)) {
                    mfaPolicyMutation.reset();
                  }
                }
              }}
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
      {adminElevation.dialog}
    </div>
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
