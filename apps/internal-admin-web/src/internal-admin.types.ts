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

export type SecurityCenterSnapshot = {
  risk_events_24h: number;
  high_risk_events_24h: number;
  active_users_without_mfa: number;
  suspended_principal_count: number;
  revoked_principal_count: number;
  unverified_user_count: number;
  active_oauth_consent_count: number;
  recent_risk_events: RecentRiskEvent[];
  users_without_mfa: UserWithoutMfa[];
};

export type ComplianceCenterSnapshot = {
  active_consent_count: number;
  revoked_consent_count_30d: number;
  suppressed_email_count: number;
  email_bounce_count_24h: number;
  email_delivery_failure_count_24h: number;
  unverified_user_count: number;
  recent_revoked_consents: RecentRevokedConsent[];
  recent_suppressed_emails: RecentSuppressedEmail[];
};

export type CommunicationsCenterSnapshot = {
  queued_message_count: number;
  sent_message_count_24h: number;
  delivered_message_count_24h: number;
  failed_message_count_24h: number;
  suppressed_email_count: number;
  webhook_event_count_24h: number;
  unprocessed_event_count: number;
  status_distribution: EmailStatusDistribution[];
  business_type_distribution: BusinessTypeDistribution[];
  recent_failures: RecentEmailFailure[];
  recent_suppressions: RecentEmailSuppression[];
  recent_unprocessed_events: RecentEmailEvent[];
};

export type IdentityGovernanceSnapshot = {
  active_idp_count: number;
  unverified_domain_count: number;
  active_scim_connector_count: number;
  overdue_access_review_count: number;
  pending_review_item_count: number;
  active_break_glass_count: number;
  pending_recovery_count: number;
  unverified_domains: UnverifiedDomain[];
  sso_providers: SsoProvider[];
  scim_connectors: ScimConnector[];
  overdue_access_reviews: OverdueAccessReview[];
  break_glass_accounts: BreakGlassAccount[];
  pending_recovery_requests: PendingRecoveryRequest[];
};

export type RegionCenterSnapshot = {
  eu_workspace_count: number;
  non_eu_workspace_count: number;
  gdpr_workspace_count: number;
  non_gdpr_workspace_count: number;
  multi_region_tenant_count: number;
  region_distribution: RegionDistribution[];
  jurisdiction_distribution: JurisdictionDistribution[];
  non_eu_workspaces: RegionWorkspace[];
  multi_region_tenants: MultiRegionTenant[];
};

export type DeveloperCenterSnapshot = {
  active_client_count: number;
  pending_marketplace_app_count: number;
  failed_webhook_delivery_count_24h: number;
  active_webhook_endpoint_count: number;
  expiring_secret_count: number;
  restricted_scope_count: number;
  failing_health_check_count: number;
  pending_marketplace_apps: PendingMarketplaceApp[];
  webhook_failures: WebhookFailure[];
  expiring_secrets: ExpiringSecret[];
  risky_scopes: RiskyScope[];
  health_issues: DeveloperHealthIssue[];
};

export type RevenueCenterSnapshot = {
  captured_payments_30d: MoneyTotal[];
  open_invoices: MoneyTotal[];
  overdue_invoices: MoneyTotal[];
  refunds_30d: MoneyTotal[];
  disputes_30d: MoneyTotal[];
  active_subscription_count: number;
  trialing_subscription_count: number;
  open_dunning_case_count: number;
  unresolved_reconciliation_difference_count: number;
  recent_overdue_invoices: RecentOverdueInvoice[];
  recent_captured_payments: RecentCapturedPayment[];
};

export type CustomerCenterSnapshot = {
  active_tenant_count: number;
  suspended_tenant_count: number;
  dormant_workspace_count: number;
  pending_invitation_count: number;
  expired_invitation_count: number;
  usage_events_24h: number;
  storage_bytes_used: number;
  file_count: number;
  high_storage_workspaces: HighStorageWorkspace[];
  dormant_workspaces: DormantWorkspace[];
  tenants_with_pending_invites: TenantPendingInvites[];
};

export type OperationsCenterSnapshot = {
  provider_event_failure_count: number;
  provider_event_backlog_count: number;
  export_pending_count: number;
  export_failed_count: number;
  reconciliation_pending_count: number;
  reconciliation_failed_count: number;
  unresolved_reconciliation_difference_count: number;
  queued_email_count: number;
  dropped_email_count_24h: number;
  audit_events_24h: number;
  recent_provider_failures: RecentProviderFailure[];
  recent_export_runs: RecentExportRun[];
  recent_reconciliation_differences: RecentReconciliationDifference[];
};

export type AccessCenterSnapshot = {
  workspace_owner_count: number;
  workspace_admin_count: number;
  ownerless_workspace_count: number;
  service_account_count: number;
  stale_service_account_count: number;
  oauth_client_count: number;
  revoked_oauth_client_count: number;
  restricted_client_policy_count: number;
  privileged_users: PrivilegedUser[];
  ownerless_workspaces: OwnerlessWorkspace[];
  stale_service_accounts: StaleServiceAccount[];
};

export type PrivilegedUser = {
  principal_id: string;
  email: string | null;
  name: string | null;
  tenant_id: string;
  tenant_name: string;
  workspace_id: string;
  workspace_name: string;
  role: string;
  updated_at: string;
};

export type OwnerlessWorkspace = {
  workspace_id: string;
  tenant_id: string;
  tenant_name: string;
  workspace_name: string;
  plan_code: string;
  created_at: string;
};

export type StaleServiceAccount = {
  principal_id: string;
  tenant_id: string;
  tenant_name: string;
  workspace_id: string | null;
  workspace_name: string | null;
  name: string;
  last_rotated_at: string | null;
  created_at: string;
};

export type RecentProviderFailure = {
  id: string;
  tenant_id: string | null;
  tenant_name: string | null;
  provider: string;
  provider_event_id: string;
  event_type: string;
  status: string;
  received_at: string;
};

export type RecentExportRun = {
  id: string;
  export_type: string;
  status: string;
  period_start: string | null;
  period_end: string | null;
  created_at: string;
  updated_at: string;
};

export type RecentReconciliationDifference = {
  id: string;
  tenant_id: string | null;
  tenant_name: string | null;
  difference_type: string;
  severity: string;
  created_at: string;
};

export type HighStorageWorkspace = {
  workspace_id: string;
  tenant_id: string;
  tenant_name: string;
  workspace_name: string;
  plan_code: string;
  used_storage_bytes: number;
  file_count: number;
  updated_at: string;
};

export type DormantWorkspace = {
  workspace_id: string;
  tenant_id: string;
  tenant_name: string;
  workspace_name: string;
  plan_code: string;
  last_usage_at: string | null;
  created_at: string;
};

export type TenantPendingInvites = {
  tenant_id: string;
  tenant_name: string;
  pending_invitation_count: number;
  oldest_invitation_at: string;
};

export type MoneyTotal = {
  currency: string;
  amount_minor: number;
  object_count: number;
};

export type RecentOverdueInvoice = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  invoice_number: string | null;
  status: string;
  currency: string;
  total_minor: number;
  due_at: string | null;
};

export type RecentCapturedPayment = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  status: string;
  currency: string;
  amount_minor: number;
  created_at: string;
};

export type RecentRevokedConsent = {
  id: string;
  principal_id: string;
  email: string | null;
  tenant_id: string;
  tenant_name: string;
  consent_type: string;
  document_version: string;
  revoked_at: string;
};

export type RecentSuppressedEmail = {
  email: string;
  reason: string;
  principal_id: string | null;
  tenant_id: string | null;
  tenant_name: string | null;
  suppressed_at: string;
};

export type EmailStatusDistribution = {
  status: string;
  message_count: number;
};

export type BusinessTypeDistribution = {
  business_type: string;
  message_count: number;
  failure_count: number;
};

export type RecentEmailFailure = {
  id: string;
  business_type: string;
  recipient_email: string;
  provider_email_id: string | null;
  status: string;
  updated_at: string;
};

export type RecentEmailSuppression = {
  email: string;
  reason: string;
  suppressed_at: string;
};

export type RecentEmailEvent = {
  id: string;
  provider_event_id: string;
  provider_email_id: string | null;
  email: string;
  event_type: string;
  occurred_at: string;
  created_at: string;
};

export type UnverifiedDomain = {
  tenant_id: string;
  tenant_name: string;
  domain: string;
  created_at: string;
  verification_expires_at: string | null;
};

export type SsoProvider = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  provider_type: string;
  name: string;
  status: string;
  issuer: string | null;
  require_signed_assertions: boolean;
  created_at: string;
};

export type ScimConnector = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  provider: string;
  status: string;
  base_url: string | null;
  created_at: string;
};

export type OverdueAccessReview = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  name: string;
  status: string;
  due_at: string;
  pending_item_count: number;
};

export type BreakGlassAccount = {
  tenant_id: string;
  tenant_name: string;
  principal_id: string;
  procedure_reference: string;
  reason: string;
  last_used_at: string | null;
  created_at: string;
};

export type PendingRecoveryRequest = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  principal_id: string;
  email: string;
  status: string;
  available_at: string;
  created_at: string;
};

export type RegionDistribution = {
  data_region: string;
  workspace_count: number;
  tenant_count: number;
};

export type JurisdictionDistribution = {
  jurisdiction: string;
  workspace_count: number;
  tenant_count: number;
};

export type RegionWorkspace = {
  workspace_id: string;
  tenant_id: string;
  tenant_name: string;
  workspace_name: string;
  data_region: string;
  jurisdiction: string;
  created_at: string;
};

export type MultiRegionTenant = {
  tenant_id: string;
  tenant_name: string;
  workspace_count: number;
  region_count: number;
  regions: string[];
};

export type PendingMarketplaceApp = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  client_id: string;
  client_name: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type WebhookFailure = {
  id: string;
  endpoint_id: string;
  endpoint_name: string;
  tenant_id: string;
  tenant_name: string;
  event_type: string;
  status: string;
  attempt_count: number;
  response_status: number | null;
  error_message: string | null;
  created_at: string;
};

export type ExpiringSecret = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  client_id: string;
  client_name: string;
  status: string;
  secret_last4: string;
  expires_at: string | null;
};

export type RiskyScope = {
  scope_key: string;
  display_name: string;
  risk: string;
  lifecycle: string;
  owner_team: string;
  updated_at: string;
};

export type DeveloperHealthIssue = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  target_type: string;
  target_id: string;
  check_kind: string;
  status: string;
  summary: string;
  checked_at: string;
};

export type RecentRiskEvent = {
  id: string;
  principal_id: string;
  email: string | null;
  tenant_id: string;
  tenant_name: string;
  event_type: string;
  risk_score: number;
  decision: string;
  created_at: string;
};

export type UserWithoutMfa = {
  principal_id: string;
  email: string;
  name: string;
  tenant_id: string;
  tenant_name: string;
  created_at: string;
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
