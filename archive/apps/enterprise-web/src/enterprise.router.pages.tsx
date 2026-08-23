import { useQuery } from '@tanstack/react-query';
import { ModuleShellPage, type ModuleShellPageProps } from './pages/ModuleShellPage';
import { AccessReviewsPage } from './pages/AccessReviewsPage';
import { DevelopersPage } from './pages/DevelopersPage';
import { OverviewPage } from './pages/OverviewPage';
import { PoliciesPage } from './pages/PoliciesPage';
import { SecurityPage } from './pages/SecurityPage';
import { SettingsPage } from './pages/SettingsPage';
import { enterpriseContextQueryOptions } from './enterprise.queries';
import { EnterpriseRouteState } from './components/EnterpriseRouteState';
import { Skeleton } from './components/ui/skeleton';

function TenantRouteGuard({ children }: { children: React.ReactNode }) {
  const { data: context, isPending, error } = useQuery(enterpriseContextQueryOptions());

  if (isPending) {
    return (
      <div className="flex flex-col gap-6">
        <Skeleton className="h-12 w-1/3 rounded-lg" />
        <Skeleton className="h-64 w-full rounded-lg" />
      </div>
    );
  }

  if (error || context?.organization_id) {
    return (
      <EnterpriseRouteState
        tone="warning"
        title="Tenant scope required"
        description="This view requires tenant-wide administrative privileges."
      />
    );
  }

  return <>{children}</>;
}

export function OverviewRoutePage() {
  return <OverviewPage />;
}

function createModuleRoutePage(props: ModuleShellPageProps, isTenantOnly = false) {
  return function ModuleRoutePage() {
    const { data: context } = useQuery(enterpriseContextQueryOptions());
    const isOrgScoped = !!context?.organization_id;

    if (isTenantOnly && isOrgScoped) {
      return (
        <EnterpriseRouteState
          tone="warning"
          title="Tenant scope required"
          description="This view requires tenant-wide administrative privileges."
        />
      );
    }

    let summary = props.summary;
    if (isOrgScoped) {
      if (props.title === 'Workspaces') {
        summary = 'Workspace inventory and organization ownership controls.';
      } else if (props.title === 'Audit logs') {
        summary = 'Organization audit trail search and compliance evidence exports.';
      } else if (props.title === 'Usage') {
        summary =
          'Organization consumption trends across identity, storage, and platform activity.';
      }
    }

    return <ModuleShellPage {...props} summary={summary} />;
  };
}

export const UsersRoutePage = createModuleRoutePage({
  title: 'Users',
  summary: 'Tenant user directory, access posture, and lifecycle operations.',
  metrics: [
    { label: 'Active users', value: 'Pending' },
    { label: 'Invitations', value: 'Pending' },
    { label: 'Review queue', value: 'Task 5', tone: 'warning' },
  ],
});

export const WorkspacesRoutePage = createModuleRoutePage({
  title: 'Workspaces',
  summary: 'Workspace inventory and tenant-level ownership controls.',
  metrics: [
    { label: 'Total workspaces', value: 'Pending' },
    { label: 'Managed regions', value: 'Pending' },
    { label: 'Policy drift', value: 'Pending' },
  ],
});

export function DevelopersRoutePage() {
  return (
    <TenantRouteGuard>
      <DevelopersPage />
    </TenantRouteGuard>
  );
}

export function PoliciesRoutePage() {
  return (
    <TenantRouteGuard>
      <PoliciesPage />
    </TenantRouteGuard>
  );
}

export function SecurityRoutePage() {
  return (
    <TenantRouteGuard>
      <SecurityPage />
    </TenantRouteGuard>
  );
}

export function AccessReviewsRoutePage() {
  return (
    <TenantRouteGuard>
      <AccessReviewsPage />
    </TenantRouteGuard>
  );
}

export const AuditLogsRoutePage = createModuleRoutePage({
  title: 'Audit logs',
  summary: 'Tenant audit trail search and compliance evidence exports.',
  metrics: [
    { label: 'Events', value: 'Pending' },
    { label: 'Retention', value: 'Pending' },
    { label: 'Export jobs', value: 'Pending' },
  ],
});

export const BillingRoutePage = createModuleRoutePage(
  {
    title: 'Billing',
    summary: 'Subscription state, invoices, payment controls, and commercial ownership.',
    metrics: [
      { label: 'Plan', value: 'Pending' },
      { label: 'Seats', value: 'Pending' },
      { label: 'Invoice status', value: 'Pending' },
    ],
  },
  true,
);

export const UsageRoutePage = createModuleRoutePage({
  title: 'Usage',
  summary: 'Tenant consumption trends across identity, storage, and platform activity.',
  metrics: [
    { label: 'Active seats', value: 'Pending' },
    { label: 'Storage', value: 'Pending' },
    { label: 'API volume', value: 'Pending' },
  ],
});

export function SettingsRoutePage() {
  return (
    <TenantRouteGuard>
      <SettingsPage />
    </TenantRouteGuard>
  );
}
