import { Banknote, ChartNoAxesCombined, FileCheck2, KeyRound, ShieldAlert } from 'lucide-react';
import type { DetailTab } from './backoffice-service.detail-tabs';
import type { TenantDetail, UserDetail, WorkspaceDetail } from './backoffice-service.types';

export function tenantDetailTabs(data: TenantDetail): DetailTab[] {
  return [
    {
      action: { hash: '#audit', label: 'Open audit' },
      description: 'Timeline cible tenant, preuves exportables et anomalies de hash.',
      icon: FileCheck2,
      id: 'audit',
      insights: [
        { label: 'Events 24h', value: formatCount(data.audit_events_24h) },
        { label: 'Target', value: data.slug },
        { label: 'Evidence', value: 'Audit center' },
      ],
      secondaryAction: { hash: '#audit-evidence-center', label: 'Evidence' },
      title: 'Audit',
    },
    {
      action: { hash: '#revenue-center', label: 'Revenue' },
      description: 'Dunning, holds invoice, disputes et routage provider pour ce tenant.',
      icon: Banknote,
      id: 'billing',
      insights: [
        {
          label: 'Open invoices',
          tone: data.open_invoice_count > 0 ? 'danger' : 'default',
          value: formatCount(data.open_invoice_count),
        },
        {
          label: 'Provider failures',
          tone: data.provider_failure_count > 0 ? 'danger' : 'default',
          value: formatCount(data.provider_failure_count),
        },
        { label: 'Kind', value: data.kind },
      ],
      secondaryAction: { hash: '#billing-platform-center', label: 'Platform' },
      title: 'Billing',
    },
    {
      action: { hash: '#security-center', label: 'Security' },
      description: 'Posture securite, risk decisions et signaux critiques du tenant.',
      icon: ShieldAlert,
      id: 'security',
      insights: [
        { label: 'Tier', value: data.security_tier },
        { label: 'Status', value: data.status },
        { label: 'Signals', value: data.provider_failure_count > 0 ? 'Review' : 'Nominal' },
      ],
      secondaryAction: { hash: '#risk-decision-center', label: 'Risk' },
      title: 'Security',
    },
    {
      action: { hash: '#usage-center', label: 'Usage' },
      description: 'Volumes, corrections de metering et impacts rollup par tenant.',
      icon: ChartNoAxesCombined,
      id: 'usage',
      insights: [
        { label: 'Workspaces', value: formatCount(data.workspace_count) },
        { label: 'Users', value: formatCount(data.user_count) },
        { label: 'Metering', value: 'Tenant scope' },
      ],
      secondaryAction: { hash: '#customer-center', label: 'Customer' },
      title: 'Usage',
    },
    {
      action: { hash: '#access-center', label: 'Access' },
      description: 'Acces, gouvernance identite et controles operateur du tenant.',
      icon: KeyRound,
      id: 'access',
      insights: [
        { label: 'Users', value: formatCount(data.user_count) },
        { label: 'Workspaces', value: formatCount(data.workspace_count) },
        { label: 'Governance', value: data.security_tier },
      ],
      secondaryAction: { hash: '#identity-governance-center', label: 'Governance' },
      title: 'Access',
    },
  ];
}

export function workspaceDetailTabs(data: WorkspaceDetail): DetailTab[] {
  return [
    {
      action: { hash: '#audit', label: 'Open audit' },
      description: 'Evenements workspace, dernier audit et preuves liees au tenant.',
      icon: FileCheck2,
      id: 'audit',
      insights: [
        { label: 'Events 24h', value: formatCount(data.audit_events_24h) },
        { label: 'Latest audit', value: formatOptionalDate(data.latest_audit_at) },
        { label: 'Tenant', value: data.tenant_name },
      ],
      secondaryAction: { hash: '#audit-evidence-center', label: 'Evidence' },
      title: 'Audit',
    },
    {
      action: { hash: '#revenue-center', label: 'Revenue' },
      description: 'Abonnements, factures ouvertes et actions finance du workspace.',
      icon: Banknote,
      id: 'billing',
      insights: [
        { label: 'Plan', value: data.plan_code },
        { label: 'Active subs', value: formatCount(data.active_subscription_count) },
        {
          label: 'Open invoices',
          tone: data.open_invoice_count > 0 ? 'danger' : 'default',
          value: formatCount(data.open_invoice_count),
        },
      ],
      secondaryAction: { hash: '#billing-platform-center', label: 'Platform' },
      title: 'Billing',
    },
    {
      action: { hash: '#security-center', label: 'Security' },
      description: 'Risques workspace, comptes de service et statut operationnel.',
      icon: ShieldAlert,
      id: 'security',
      insights: [
        { label: 'Status', value: data.status },
        { label: 'Service accounts', value: formatCount(data.service_account_count) },
        { label: 'Owners', value: formatCount(data.owner_count) },
      ],
      secondaryAction: { hash: '#risk-decision-center', label: 'Risk' },
      title: 'Security',
    },
    {
      action: { hash: '#usage-center', label: 'Usage' },
      description: 'Corrections usage, freeze meter et replay rollup sur ce workspace.',
      icon: ChartNoAxesCombined,
      id: 'usage',
      insights: [
        { label: 'Members', value: formatCount(data.member_count) },
        { label: 'Active members', value: formatCount(data.active_member_count) },
        { label: 'Type', value: data.workspace_type },
      ],
      title: 'Usage',
    },
    {
      action: { hash: '#access-center', label: 'Access' },
      description: 'Memberships, owners et suspensions d acces sur ce workspace.',
      icon: KeyRound,
      id: 'access',
      insights: [
        { label: 'Members', value: formatCount(data.member_count) },
        { label: 'Owners', value: formatCount(data.owner_count) },
        { label: 'Active access', value: formatCount(data.active_member_count) },
      ],
      secondaryAction: { hash: '#identity-governance-center', label: 'Governance' },
      title: 'Access',
    },
  ];
}

export function userDetailTabs(data: UserDetail): DetailTab[] {
  return [
    {
      action: { hash: '#audit', label: 'Open audit' },
      description: 'Timeline user, derniers changements et evidence liee au principal.',
      icon: FileCheck2,
      id: 'audit',
      insights: [
        { label: 'Events 24h', value: formatCount(data.audit_events_24h) },
        { label: 'Latest audit', value: formatOptionalDate(data.latest_audit_at) },
        { label: 'Principal', value: shortId(data.principal_id) },
      ],
      secondaryAction: { hash: '#audit-evidence-center', label: 'Evidence' },
      title: 'Audit',
    },
    {
      action: { hash: '#revenue-center', label: 'Revenue' },
      description: 'Contexte finance indirect via tenant et workspace principal.',
      icon: Banknote,
      id: 'billing',
      insights: [
        { label: 'Tenant', value: data.tenant_name },
        { label: 'Primary workspace', value: data.primary_workspace_name ?? '-' },
        { label: 'Email verified', value: data.email_verified_at ? 'Verified' : 'Missing' },
      ],
      title: 'Billing',
    },
    {
      action: { hash: '#security-center', label: 'Security' },
      description: 'MFA, OAuth consents, signaux risk et decisions sensibles du user.',
      icon: ShieldAlert,
      id: 'security',
      insights: [
        {
          label: 'MFA active',
          tone: data.active_mfa_factor_count === 0 ? 'danger' : 'default',
          value: formatCount(data.active_mfa_factor_count),
        },
        { label: 'OAuth consents', value: formatCount(data.active_oauth_consent_count) },
        {
          label: 'Risk 24h',
          tone: data.risk_events_24h > 0 ? 'danger' : 'default',
          value: formatCount(data.risk_events_24h),
        },
      ],
      secondaryAction: { hash: '#risk-decision-center', label: 'Risk' },
      title: 'Security',
    },
    {
      action: { hash: '#usage-center', label: 'Usage' },
      description: 'Activite user par workspace et impact usage du compte.',
      icon: ChartNoAxesCombined,
      id: 'usage',
      insights: [
        { label: 'Workspaces', value: formatCount(data.workspace_count) },
        { label: 'Active access', value: formatCount(data.active_workspace_count) },
        { label: 'Primary workspace', value: data.primary_workspace_name ?? '-' },
      ],
      title: 'Usage',
    },
    {
      action: { hash: '#access-center', label: 'Access' },
      description: 'Memberships, gouvernance identite et etat principal/user.',
      icon: KeyRound,
      id: 'access',
      insights: [
        { label: 'Principal', value: data.principal_status },
        { label: 'User', value: data.user_status },
        { label: 'Active access', value: formatCount(data.active_workspace_count) },
      ],
      secondaryAction: { hash: '#identity-governance-center', label: 'Governance' },
      title: 'Access',
    },
  ];
}

function formatCount(value: number): string {
  return new Intl.NumberFormat('fr-FR').format(value);
}

function formatOptionalDate(value: string | null): string {
  if (!value) return '-';
  return new Intl.DateTimeFormat('fr-FR', {
    dateStyle: 'short',
    timeStyle: 'short',
  }).format(new Date(value));
}

function shortId(value: string): string {
  return value.length > 12 ? `${value.slice(0, 8)}...` : value;
}
