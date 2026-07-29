import { readFile } from 'node:fs/promises';

const requirements = [
  {
    file: 'apps/account-worker/src/identity.worker.audit_anchor.rs',
    evidence: [
      'audit_external_anchors',
      'snapshot_digest_sha256',
      'verify_signature',
      'put_object',
    ],
  },
  {
    file: 'apps/account-worker/src/identity.worker.audit_anchor.kms.rs',
    evidence: ['key-manager/v1alpha1/regions', 'sign_digest', 'verify_signature'],
  },
  {
    file: 'infrastructure/modules/scaleway-audit-archive/main.tf',
    evidence: [
      'object_lock_enabled = true',
      'mode  = "COMPLIANCE"',
      'prevent_destroy = true',
      's3:PutObject',
      'anchors/*',
    ],
  },
  {
    file: 'infrastructure/modules/scaleway-audit-archive/main.tf',
    evidence: ['ObjectStorageObjectsWrite'],
    forbidden: ['ObjectStorageObjectsDelete'],
  },
  {
    file: 'infrastructure/modules/scaleway-v1/iam.tf',
    evidence: ['KeyManagerKeySign', 'KeyManagerKeyVerify'],
    forbidden: ['ObjectStorageObjectsWrite', 'ObjectStorageObjectsDelete'],
  },
  {
    file: 'infrastructure/modules/scaleway-v1/network.tf',
    evidence: [
      'outbound_default_policy = "drop"',
      'var.enable_jit_ssh ? var.ssh_allowed_ips : []',
      'edge_allowed_ipv4_cidrs',
    ],
  },
  {
    file: 'infrastructure/environments/production/main.tf',
    evidence: [
      'http_request_firewall_managed',
      '4814384a9e5d4991b9815dcfc25d2f1f',
      'enable_external_audit_archive         = true',
      'enable_jit_ssh                 = var.enable_jit_ssh',
      'audit_archive_writer_access_key       = var.audit_archive_writer_access_key',
    ],
  },
  {
    file: 'infrastructure/environments/production/variables.tf',
    evidence: [
      'endswith(cidr, "/32")',
      'var.jit_access_requester != var.jit_access_approver',
      'jit_access_authentication_method',
      'jit_access_authentication_event_ref',
      'jit_access_authentication_verified_at',
      'timeadd(timestamp(), "-15m")',
      'timeadd(timestamp(), "60m")',
      'default     = false',
    ],
  },
  {
    file: 'apps/backoffice-service/src/internal_admin.privileged_authentication.rs',
    evidence: [
      'phishing_resistant_authentication_required',
      'PRIVILEGED_AUTHENTICATION_MAX_AGE_SECONDS',
      'x-nvbes-authentication-event-id',
      'privileged_authentication_enforced',
    ],
  },
  {
    file: 'apps/enterprise-service/src/enterprise.grpc.privileged_authentication.rs',
    evidence: [
      'PrivilegedAuthenticationContext',
      'has_recent_phishing_resistant_authentication',
      'authentication_event_id',
    ],
  },
  {
    file: 'apps/account-service/src/identity.domains.auth.mfa.policy.rs',
    evidence: ['privileged_passkey_required', 'vec!["webauthn".to_string()]'],
  },
  {
    file: 'apps/billing-service/src/billing.auth.rs',
    evidence: [
      'has_recent_phishing_resistant_authentication',
      'authentication_event_id',
      'privileged_authentication_enforced',
    ],
  },
  {
    file: 'apps/developer-service/src/developer.http.auth.rs',
    evidence: [
      'has_recent_phishing_resistant_authentication',
      'authentication_event_id',
      'privileged_authentication_enforced',
    ],
  },
  {
    file: 'apps/enterprise-service/src/enterprise.grpc.admin_elevation.rs',
    evidence: ['authentication_evidence', 'enterprise.admin_elevation.granted'],
  },
  {
    file: 'apps/enterprise-service/src/enterprise.grpc.break_glass.rs',
    evidence: ['enterprise.break_glass.activated', 'authentication_event_id'],
  },
  {
    file: 'infrastructure/environments/security-audit-archive/main.tf',
    evidence: ['source = "../../modules/scaleway-audit-archive"', 'retention_years = 7'],
  },
  {
    file: 'infrastructure/environments/security-audit-archive/versions.tf',
    evidence: ['profile    = var.security_scaleway_profile'],
    forbidden: ['scaleway_production_profile'],
  },
  {
    file: 'infrastructure/environments/production/alloy.config.alloy',
    evidence: [
      'redacted_secret',
      'FilteredAuthorization',
      'FilteredEmail',
      'FilteredIp',
      'oversized_log_line',
    ],
    forbidden: ['baseline_log_sampling'],
  },
  {
    file: 'infrastructure/local/observability/grafana-provisioning/alerting/nvbes-security-identity.yml',
    evidence: [
      'nvbes-siem-account-takeover',
      'nvbes-siem-refresh-token-reuse',
      'nvbes-siem-privilege-elevation',
    ],
  },
  {
    file: 'infrastructure/local/observability/grafana-provisioning/alerting/nvbes-security-data.yml',
    evidence: [
      'nvbes-siem-sensitive-export',
      'nvbes-siem-destructive-action',
      'nvbes-siem-cross-tenant-access',
    ],
  },
  {
    file: 'infrastructure/environments/production/siem.tf',
    evidence: [
      'grafana_rule_group',
      'nvbes-security-identity.yml',
      'nvbes-security-data.yml',
      'grafana_loki_datasource_uid',
    ],
  },
  {
    file: 'docs/security/security-operations-inventory.yml',
    evidence: [
      'identity-provider',
      'audit-evidence',
      'software-delivery',
      'subprocessors:',
      'required_evidence:',
    ],
  },
  {
    file: 'docs/operations/security-incident-exercises.md',
    evidence: ['Identity Provider compromise', 'KMS compromise', 'CI compromise', 'tenant isolation'],
  },
  {
    file: 'docs/operations/production-access-jit.md',
    evidence: ['enable_jit_ssh = false', 'durée maximale de 60 minutes', 'Post-access review'],
  },
  {
    file: 'docs/compliance/independent-pentest-policy.md',
    evidence: ['avant la première ouverture publique', 'annuellement', 'retest externe', 'Bug bounty'],
  },
];

const failures = [];

for (const requirement of requirements) {
  const content = await readFile(requirement.file, 'utf8');
  for (const evidence of requirement.evidence) {
    if (!content.includes(evidence)) {
      failures.push(`${requirement.file}: missing evidence "${evidence}"`);
    }
  }
  for (const forbidden of requirement.forbidden ?? []) {
    if (content.includes(forbidden)) {
      failures.push(`${requirement.file}: forbidden evidence "${forbidden}"`);
    }
  }
}

const assurance = JSON.parse(
  await readFile('docs/compliance/external-security-assurance.json', 'utf8'),
);
const pentest = assurance.independent_pentest;
const privilegedAccess = assurance.privileged_phishing_resistant_access;
const privilegedScopes = Object.values(privilegedAccess?.scopes ?? {});
if (process.env.NVBES_PRODUCTION_RELEASE === '1') {
  const validUntil = Date.parse(pentest.valid_until ?? '');
  if (
    assurance.production_activation !== 'approved' ||
    pentest.status !== 'passed' ||
    pentest.critical_open !== 0 ||
    pentest.high_open !== 0 ||
    pentest.retest_status !== 'passed' ||
    !pentest.report_reference?.startsWith('vault://') ||
    !Number.isFinite(validUntil) ||
    validUntil <= Date.now()
  ) {
    failures.push(
      'production release requires a valid independent pentest, zero open critical/high findings, passed retest, and an evidence-vault reference',
    );
  }
  if (
    privilegedAccess?.status !== 'passed' ||
    privilegedAccess.tested_release !== pentest.tested_release ||
    !privilegedAccess.completed_at ||
    !privilegedAccess.evidence_reference?.startsWith('vault://') ||
    privilegedScopes.length !== 5 ||
    privilegedScopes.some((status) => status !== 'passed')
  ) {
    failures.push(
      'production release requires signed live evidence that phishing-resistant authentication is enforced for backoffice/support, billing/secrets, production access, elevation, and break-glass',
    );
  }
}

if (failures.length > 0) {
  console.error('Production security operations gate failed:');
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log(`Production security operations gate passed (${requirements.length} controls).`);
