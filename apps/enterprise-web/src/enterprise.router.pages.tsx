import { ModuleShellPage, type ModuleShellPageProps } from './pages/ModuleShellPage';
import { AccessReviewsPage } from './pages/AccessReviewsPage';
import { DevelopersPage } from './pages/DevelopersPage';
import { OverviewPage } from './pages/OverviewPage';
import { PoliciesPage } from './pages/PoliciesPage';
import { SecurityPage } from './pages/SecurityPage';
import { SettingsPage } from './pages/SettingsPage';

export function OverviewRoutePage() {
  return <OverviewPage />;
}

function createModuleRoutePage(props: ModuleShellPageProps) {
  return function ModuleRoutePage() {
    return <ModuleShellPage {...props} />;
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
  return <DevelopersPage />;
}

export function PoliciesRoutePage() {
  return <PoliciesPage />;
}

export function SecurityRoutePage() {
  return <SecurityPage />;
}

export function AccessReviewsRoutePage() {
  return <AccessReviewsPage />;
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

export const BillingRoutePage = createModuleRoutePage({
  title: 'Billing',
  summary: 'Subscription state, invoices, payment controls, and commercial ownership.',
  metrics: [
    { label: 'Plan', value: 'Pending' },
    { label: 'Seats', value: 'Pending' },
    { label: 'Invoice status', value: 'Pending' },
  ],
});

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
  return <SettingsPage />;
}
