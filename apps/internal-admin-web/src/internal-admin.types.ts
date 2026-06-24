export type AdminCredentials = {
  workspaceId: string;
  internalToken: string;
  actorPrincipalId: string;
  backofficeRole: string;
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

export type TenantLifecycleRequest = {
  confirm_code: string;
  reason: string;
};

export type TenantLifecycleResult = {
  tenant_id: string;
  previous_status: string;
  next_status: string;
  audit_action: string;
};

export type WorkspaceDetail = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  name: string;
  status: string;
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

export type WorkspaceLifecycleRequest = {
  confirm_code: string;
  reason: string;
};

export type WorkspaceLifecycleResult = {
  workspace_id: string;
  tenant_id: string;
  previous_status: string;
  next_status: string;
  audit_action: string;
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

export type UserLifecycleRequest = {
  confirm_code: string;
  reason: string;
};

export type UserLifecycleResult = {
  principal_id: string;
  tenant_id: string;
  previous_principal_status: string;
  previous_user_status: string;
  next_principal_status: string;
  next_user_status: string;
  audit_action: string;
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
  active_mfa_factors: ActiveMfaFactor[];
  active_oauth_consents: ActiveOauthConsent[];
};

export type RiskDecisionSnapshot = {
  identity_risk_event_count_24h: number;
  high_identity_risk_event_count_24h: number;
  billing_risk_signal_count_24h: number;
  high_billing_risk_score_count: number;
  active_access_policy_count: number;
  access_policy_count_24h: number;
  recent_identity_risks: IdentityRiskDecision[];
  billing_risk_scores: BillingRiskScore[];
  billing_risk_signals: BillingRiskSignal[];
  active_access_policies: AccessPolicySnapshot[];
};

export type RiskActionRequest = {
  confirm_code: string;
  reason: string;
};

export type RiskActionResult = {
  object_id: string;
  action_kind: string;
  status: string;
  audit_action: string;
};

export type AuditEvidenceSnapshot = {
  audit_events_24h: number;
  actorless_event_count_24h: number;
  sensitive_action_count_24h: number;
  missing_hash_count: number;
  backfilled_hash_count: number;
  active_signing_key_count: number;
  deprecated_signing_key_count: number;
  revoked_signing_key_count: number;
  recent_audit_events: AuditEvidenceEvent[];
  actorless_events: AuditEvidenceEvent[];
  hash_anomalies: AuditHashAnomaly[];
  signing_keys: SigningKeySummary[];
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

export type ComplianceActionRequest = {
  confirm_code: string;
  reason: string;
};

export type SuppressionReviewRequest = {
  confirm_code: string;
  email: string;
  reason: string;
};

export type ComplianceActionResult = {
  object_id: string;
  action_kind: string;
  status: string;
  audit_action: string;
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

export type CommunicationsActionRequest = {
  confirm_code: string;
  reason: string;
};

export type EmailSuppressionRequest = {
  confirm_code: string;
  email: string;
  reason: string;
};

export type CommunicationsActionResult = {
  object_id: string;
  action_kind: string;
  status: string;
  audit_action: string;
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

export type RegionFlagRequest = {
  confirm_code: string;
  data_region: string;
  jurisdiction: string;
  reason: string;
};

export type RegionExceptionRequest = {
  confirm_code: string;
  exception_kind: string;
  reason: string;
};

export type RegionActionResult = {
  object_id: string;
  action_kind: string;
  status: string;
  audit_action: string;
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

export type DeveloperActionRequest = {
  confirm_code: string;
  reason: string;
};

export type DeveloperActionResult = {
  object_id: string;
  action_kind: string;
  status: string;
  audit_action: string;
};

export type EntitlementsSnapshot = {
  active_plan_count: number;
  active_feature_count: number;
  quota_definition_count: number;
  active_entitlement_count: number;
  over_quota_balance_count: number;
  unpublished_change_count: number;
  active_trial_grant_count: number;
  active_plans: EntitlementPlan[];
  over_quota_balances: OverQuotaBalance[];
  expiring_entitlements: ExpiringEntitlement[];
  unpublished_changes: UnpublishedEntitlementChange[];
};

export type EntitlementFeatureActionRequest = {
  confirm_code: string;
  feature_code: string;
  value: Record<string, unknown>;
  reason: string;
};

export type EntitlementQuotaOverrideRequest = {
  confirm_code: string;
  quota_code: string;
  included_quantity: number;
  reason: string;
};

export type EntitlementPublishRequest = {
  confirm_code: string;
  reason: string;
};

export type EntitlementActionResult = {
  object_id: string;
  action_kind: string;
  status: string;
  published_change_count: number;
  audit_action: string;
};

export type UsageCenterSnapshot = {
  active_meter_count: number;
  usage_event_count_24h: number;
  usage_quantity_24h: number;
  correction_count_30d: number;
  rollup_count_current_period: number;
  distinct_tenant_count_24h: number;
  meter_usage_24h: MeterUsage[];
  tenant_usage_24h: TenantUsage[];
  recent_rollups: UsageRollup[];
  recent_corrections: UsageCorrection[];
};

export type UsageCorrectionRequest = {
  confirm_code: string;
  usage_event_id: string | null;
  meter_code: string;
  quantity_delta: number;
  reason: string;
};

export type FreezeMeterRequest = {
  confirm_code: string;
  meter_code: string;
  reason: string;
};

export type ReplayUsageRollupRequest = {
  confirm_code: string;
  reason: string;
};

export type UsageActionResult = {
  object_id: string;
  action_kind: string;
  status: string;
  audit_action: string;
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
  recent_dunning_cases: RecentDunningCase[];
  recent_disputes: RecentDispute[];
  recent_overdue_invoices: RecentOverdueInvoice[];
  recent_captured_payments: RecentCapturedPayment[];
};

export type RevenueActionRequest = {
  confirm_code: string;
  reason: string;
};

export type RevenueActionResult = {
  object_id: string;
  action_kind: string;
  status: string;
  audit_action: string;
};

export type BillingPlatformSnapshot = {
  active_provider_count: number;
  active_provider_account_count: number;
  active_routing_rule_count: number;
  fallback_routing_rule_count: number;
  planned_migration_count: number;
  pending_kyc_profile_count: number;
  region_policy_count: number;
  active_einvoicing_profile_count: number;
  providers: BillingProviderSummary[];
  routing_rules: ProviderRoutingRule[];
  provider_migrations: ProviderMigrationRun[];
  kyc_profiles: KycProfile[];
  region_policies: BillingRegionPolicy[];
  einvoicing_profiles: EinvoicingProfile[];
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

export type OperationsActionRequest = {
  confirm_code: string;
  reason: string;
};

export type OperationsActionResult = {
  object_id: string;
  action_kind: string;
  status: string;
  audit_action: string;
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

export type AccessActionRequest = {
  confirm_code: string;
  reason: string;
};

export type AccessActionResult = {
  workspace_id: string;
  tenant_id: string;
  principal_id: string;
  previous_status: string;
  next_status: string;
  audit_action: string;
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

export type RecentDunningCase = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  status: string;
  policy_state: string;
  opened_at: string;
};

export type RecentDispute = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  status: string;
  currency: string;
  amount_minor: number;
  created_at: string;
};

export type BillingProviderSummary = {
  provider: string;
  status: string;
  account_count: number;
  updated_at: string;
};

export type ProviderRoutingRule = {
  id: string;
  priority: number;
  provider: string;
  country: string | null;
  currency: string | null;
  payment_method: string | null;
  customer_type: string | null;
  fallback_enabled: boolean;
  status: string;
};

export type ProviderMigrationRun = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  from_provider: string;
  to_provider: string;
  status: string;
  started_at: string | null;
  updated_at: string;
};

export type KycProfile = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  company_name: string | null;
  company_domain: string | null;
  vat_id: string | null;
  proof_reference: string | null;
  updated_at: string;
};

export type BillingRegionPolicy = {
  id: string;
  country: string;
  currency: string;
  allowed_payment_methods: string[];
  invoice_retention_years: number;
  tax_evidence_required: boolean;
  einvoicing_profile_code: string | null;
};

export type EinvoicingProfile = {
  id: string;
  code: string;
  country: string | null;
  format: string;
  status: string;
  updated_at: string;
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

export type EntitlementPlan = {
  plan_id: string;
  product_name: string;
  plan_code: string;
  plan_name: string;
  active_version_count: number;
  feature_count: number;
};

export type OverQuotaBalance = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  workspace_id: string | null;
  workspace_name: string | null;
  quota_code: string;
  included_quantity: number;
  used_quantity: number;
  period_end: string;
};

export type ExpiringEntitlement = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  workspace_id: string | null;
  workspace_name: string | null;
  status: string;
  effective_to: string;
};

export type UnpublishedEntitlementChange = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  event_id: string;
  created_at: string;
};

export type MeterUsage = {
  meter_code: string;
  unit: string;
  event_count: number;
  quantity: number;
};

export type TenantUsage = {
  tenant_id: string;
  tenant_name: string;
  event_count: number;
  quantity: number;
};

export type UsageRollup = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  workspace_id: string | null;
  workspace_name: string | null;
  meter_code: string;
  quantity: number;
  unit: string;
  period_start: string;
  period_end: string;
};

export type UsageCorrection = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  meter_code: string;
  quantity_delta: number;
  reason: string;
  created_by_principal_id: string | null;
  created_at: string;
};

export type GovernanceActionRequest = {
  confirm_code: string;
  reason: string;
};

export type GovernanceActionResult = {
  object_id: string;
  tenant_id: string;
  principal_id: string;
  audit_action: string;
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

export type AuditEvidenceEvent = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  workspace_id: string | null;
  actor_principal_id: string | null;
  actor_email: string | null;
  action: string;
  target_type: string;
  target_id: string | null;
  ip: string | null;
  created_at: string;
};

export type AuditHashAnomaly = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  action: string;
  event_hash: string;
  previous_event_hash: string | null;
  created_at: string;
};

export type SigningKeySummary = {
  kid: string;
  algorithm: string;
  status: string;
  kms_key_id: string | null;
  activated_at: string;
  deprecated_at: string | null;
  revoked_at: string | null;
};

export type IdentityRiskDecision = {
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

export type BillingRiskScore = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  score: number;
  decision: string;
  created_at: string;
};

export type BillingRiskSignal = {
  id: string;
  tenant_id: string | null;
  tenant_name: string | null;
  signal_type: string;
  occurred_at: string;
};

export type AccessPolicySnapshot = {
  id: string;
  tenant_id: string;
  tenant_name: string;
  workspace_id: string | null;
  workspace_name: string | null;
  policy_state: string;
  reason: string;
  effective_from: string;
  effective_to: string | null;
};

export type UserWithoutMfa = {
  principal_id: string;
  email: string;
  name: string;
  tenant_id: string;
  tenant_name: string;
  created_at: string;
};

export type ActiveMfaFactor = {
  id: string;
  principal_id: string;
  email: string;
  tenant_id: string;
  tenant_name: string;
  factor_type: string;
  label: string | null;
  last_used_at: string | null;
  created_at: string;
};

export type ActiveOauthConsent = {
  id: string;
  principal_id: string;
  email: string;
  tenant_id: string;
  tenant_name: string;
  workspace_id: string | null;
  workspace_name: string | null;
  client_id: string;
  scopes: string[];
  granted_at: string;
  expires_at: string | null;
};

export type SecurityActionRequest = {
  confirm_code: string;
  reason: string;
};

export type SecurityActionResult = {
  object_id: string;
  tenant_id: string;
  principal_id: string;
  audit_action: string;
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

export type RunbookExecutionRequest = {
  confirm_code: string;
  reason: string;
};

export type RunbookExecutionResult = {
  workspace_id: string;
  tenant_id: string;
  runbook_id: string;
  audit_action: string;
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

export type AuditEventFilters = {
  action?: string;
  targetType?: string;
  query?: string;
  limit?: number;
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
  confirm_code: string;
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
  confirm_code: string;
  provider: string;
  provider_event_id: string;
  reason: string;
};

export type ProviderMigrationRequest = {
  confirm_code: string;
  from_provider: string;
  to_provider: string;
  reason: string;
};

export type GraceOverrideRequest = {
  confirm_code: string;
  subscription_id?: string;
  grace_days: number;
  reason: string;
};

export type ManualCompRequest = {
  confirm_code: string;
  amount_minor: number;
  currency: string;
  direction: string;
  reason: string;
};

export type ExportType = 'invoices' | 'payments' | 'tax' | 'ledger' | 'customers' | 'subscriptions';
