import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { Code2, KeyRound, RotateCcw } from 'lucide-react';
import { Badge } from '../components/ui/badge';
import { Card, CardContent } from '../components/ui/card';
import { enterpriseClient } from '../enterprise.api';
import {
  enterpriseContextQueryOptions,
  enterpriseDevelopersQueryOptions,
  enterpriseQueryKeys,
} from '../enterprise.queries';
import { CredentialsCard, needsRotation, rotationWindowDays } from './DevelopersPage.credentials';
import { DevelopersErrorAlert, DevelopersLoadingState } from './DevelopersPage.states';
import { isAdminElevationCancelled, useAdminElevation } from './UsersPage.admin-elevation';

export function DevelopersPage() {
  const queryClient = useQueryClient();
  const contextQuery = useQuery(enterpriseContextQueryOptions());
  const developersQuery = useQuery(enterpriseDevelopersQueryOptions());
  const credentials = developersQuery.data?.credentials ?? [];
  const rotationCandidates = credentials.filter(needsRotation);
  const adminElevation = useAdminElevation({
    active: contextQuery.data?.admin_elevation.active ?? false,
    breakGlass: contextQuery.data?.break_glass ?? null,
    onGranted: () => queryClient.invalidateQueries({ queryKey: enterpriseQueryKeys.context }),
  });
  const revokeMutation = useMutation({
    mutationFn: (credentialId: string) =>
      adminElevation.runElevated(() =>
        enterpriseClient.revokeEnterpriseDeveloperSecret(credentialId),
      ),
    onSuccess: (data) => {
      queryClient.setQueryData(enterpriseQueryKeys.developers, data);
    },
  });

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

      {(developersQuery.error ?? contextQuery.error) ? (
        <DevelopersErrorAlert error={developersQuery.error ?? contextQuery.error} />
      ) : null}

      {developersQuery.isPending || contextQuery.isPending ? (
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

          <CredentialsCard
            credentials={credentials}
            pendingCredentialId={
              revokeMutation.isPending ? (revokeMutation.variables ?? null) : null
            }
            revokeError={revokeMutation.error}
            pending={revokeMutation.isPending || adminElevation.pending}
            onRevoke={async (credentialId) => {
              try {
                await revokeMutation.mutateAsync(credentialId);
              } catch (error) {
                if (isAdminElevationCancelled(error)) {
                  revokeMutation.reset();
                }
              }
            }}
          />
        </>
      )}

      {adminElevation.dialog}
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
