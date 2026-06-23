export type AdminCredentials = {
  workspaceId: string;
  internalToken: string;
  actorPrincipalId: string;
};

export type SearchResult = {
  kind: string;
  id: string;
  label: string;
  status: string;
};

export type GlobalSearchResult = {
  kind: string;
  id: string;
  label: string;
  status: string;
  tenant_id: string | null;
  workspace_id: string | null;
};

export type TenantDetail = {
  id: string;
  name: string;
  slug: string;
  status: string;
  kind: string;
  security_tier: string;
  workspace_count: number;
  user_count: number;
  audit_events_24h: number;
  open_invoice_count: number;
  provider_failure_count: number;
  created_at: string;
  updated_at: string;
};

export type WorkspaceDetail = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  name: string;
  workspace_type: string;
  plan_code: string;
  trial_ends_at: string | null;
  member_count: number;
  owner_count: number;
  active_member_count: number;
  service_account_count: number;
  audit_events_24h: number;
  open_invoice_count: number;
  active_subscription_count: number;
  latest_audit_at: string | null;
  created_at: string;
  updated_at: string;
};

export type UserDetail = {
  principal_id: string;
  tenant_id: string;
  tenant_name: string;
  email: string;
  name: string;
  principal_status: string;
  user_status: string;
  email_verified_at: string | null;
  workspace_count: number;
  active_workspace_count: number;
  active_mfa_factor_count: number;
  active_oauth_consent_count: number;
  risk_events_24h: number;
  audit_events_24h: number;
  latest_risk_at: string | null;
  latest_audit_at: string | null;
  primary_workspace_id: string | null;
  primary_workspace_name: string | null;
  created_at: string;
  updated_at: string;
};

export type MutationResult = {
  object_id: string;
  ledger_entry_count: number;
  audit_action: string;
};

export type ProviderReplayResult = {
  object_id: string;
  provider: string;
  provider_event_id: string;
  status: string;
  audit_action: string;
};

export type BillingRunbook = {
  id: string;
  title: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  steps: string[];
};

export type AuditEvent = {
  id: string;
  action: string;
  actor_principal_id: string | null;
  actor_email: string | null;
  target_type: string;
  target_id: string | null;
  metadata: unknown;
  created_at: string;
};

export type BillingOverview = {
  open_invoice_count: number;
  overdue_invoice_count: number;
  open_invoice_total_minor: number;
  failed_provider_event_count: number;
  pending_refund_count: number;
  active_subscription_count: number;
  captured_payment_total_minor_30d: number;
  last_billing_audit_at: string | null;
};

export type CommandCenterSnapshot = {
  tenant_count: number;
  workspace_count: number;
  user_count: number;
  audit_events_24h: number;
  billing_provider_failures: number;
  overdue_invoice_count: number;
  failed_payment_count: number;
  latest_audit_at: string | null;
};

export type ProviderEventFailure = {
  id: string;
  provider: string;
  provider_event_id: string;
  event_type: string;
  status: string;
  signature_valid: boolean;
  payload_summary: unknown;
  received_at: string;
  processed_at: string | null;
};

export type CreditNoteRequest = {
  invoice_id: string;
  amount_minor: number;
  currency: string;
  reason: string;
};

export type RefundIntentRequest = CreditNoteRequest & {
  payment_id: string;
  provider: string;
};

export type ProviderReplayRequest = {
  provider: string;
  provider_event_id: string;
  reason: string;
};

export type ProviderMigrationRequest = {
  from_provider: string;
  to_provider: string;
  reason: string;
};

export type GraceOverrideRequest = {
  subscription_id?: string;
  grace_days: number;
  reason: string;
};

export type ManualCompRequest = {
  amount_minor: number;
  currency: string;
  direction: string;
  reason: string;
};

export type ExportType = 'invoices' | 'payments' | 'tax' | 'ledger' | 'customers' | 'subscriptions';
