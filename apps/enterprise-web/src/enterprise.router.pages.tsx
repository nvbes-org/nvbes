import { ModuleShellPage, type ModuleShellPageProps } from './pages/ModuleShellPage';
import { OverviewPage } from './pages/OverviewPage';

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

export const DevelopersRoutePage = createModuleRoutePage({
  title: 'Developers',
  summary: 'Service accounts, application access, and developer platform controls.',
  metrics: [
    { label: 'Apps', value: 'Pending' },
    { label: 'Service accounts', value: 'Pending' },
    { label: 'Key rotation', value: 'Pending' },
  ],
});

export const PoliciesRoutePage = createModuleRoutePage({
  title: 'Policies',
  summary: 'Central policy configuration for identity, data, and workspace governance.',
  metrics: [
    { label: 'Active policies', value: 'Pending' },
    { label: 'Exceptions', value: 'Pending' },
    { label: 'Pending review', value: 'Pending', tone: 'warning' },
  ],
});

export const SecurityRoutePage = createModuleRoutePage({
  title: 'Security',
  summary: 'Security posture, enforcement coverage, and tenant risk indicators.',
  metrics: [
    { label: 'MFA coverage', value: 'Pending' },
    { label: 'Risk events', value: 'Pending' },
    { label: 'Open findings', value: 'Pending', tone: 'warning' },
  ],
});

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

export const SettingsRoutePage = createModuleRoutePage({
  title: 'Settings',
  summary: 'Tenant profile, defaults, regional posture, and administrative preferences.',
  metrics: [
    { label: 'Tenant', value: 'Pending' },
    { label: 'Primary region', value: 'Pending' },
    { label: 'Admin contacts', value: 'Pending' },
  ],
});
