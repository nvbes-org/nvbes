variable "scaleway_project_id" {
  description = "Scaleway project ID for production workloads."
  type        = string
}

variable "scaleway_production_profile" {
  description = "Local/CI Scaleway profile restricted to the production workload account."
  type        = string
}

variable "enable_jit_ssh" {
  description = "Emergency-only production SSH gate. Enabling it requires the complete time-bound approval contract below."
  type        = bool
  default     = false

  validation {
    condition = !var.enable_jit_ssh || (
      length(var.ssh_allowed_ips) > 0 &&
      alltrue([
        for cidr in var.ssh_allowed_ips :
        can(cidrnetmask(cidr)) && endswith(cidr, "/32")
      ]) &&
      length(trimspace(coalesce(var.jit_access_ticket, ""))) > 0 &&
      length(trimspace(coalesce(var.jit_access_requester, ""))) > 0 &&
      length(trimspace(coalesce(var.jit_access_approver, ""))) > 0 &&
      var.jit_access_requester != var.jit_access_approver &&
      contains(["webauthn", "passkey", "security_key"], coalesce(var.jit_access_authentication_method, "")) &&
      startswith(coalesce(var.jit_access_authentication_event_ref, ""), "audit://") &&
      try(timecmp(var.jit_access_authentication_verified_at, timestamp()) <= 0, false) &&
      try(timecmp(var.jit_access_authentication_verified_at, timeadd(timestamp(), "-15m")) >= 0, false) &&
      try(timecmp(var.jit_access_expires_at, timestamp()) > 0, false) &&
      try(timecmp(var.jit_access_expires_at, timeadd(timestamp(), "60m")) <= 0, false)
    )
    error_message = "JIT SSH requires /32 source CIDRs, a ticket, distinct requester/approver, a recent audit-linked WebAuthn authentication, and an expiry within the next 60 minutes."
  }
}

variable "ssh_allowed_ips" {
  description = "Exact /32 source CIDRs for the approved JIT access window."
  type        = list(string)
  default     = []
}

variable "jit_access_ticket" {
  description = "Incident or change ticket authorizing temporary production access."
  type        = string
  default     = null
  nullable    = true
}

variable "jit_access_requester" {
  description = "Named human requesting temporary production access."
  type        = string
  default     = null
  nullable    = true
}

variable "jit_access_approver" {
  description = "Named independent approver for temporary production access."
  type        = string
  default     = null
  nullable    = true
}

variable "jit_access_expires_at" {
  description = "RFC3339 expiry no more than 60 minutes after the Terraform operation."
  type        = string
  default     = null
  nullable    = true
}

variable "jit_access_authentication_method" {
  description = "Phishing-resistant authentication method used by the requester: webauthn, passkey, or security_key."
  type        = string
  default     = null
  nullable    = true
}

variable "jit_access_authentication_event_ref" {
  description = "Immutable audit:// reference for the successful privileged authentication event."
  type        = string
  default     = null
  nullable    = true
}

variable "jit_access_authentication_verified_at" {
  description = "RFC3339 time of the phishing-resistant authentication, no more than 15 minutes before the Terraform operation."
  type        = string
  default     = null
  nullable    = true
}

variable "scaleway_region" {
  type    = string
  default = "fr-par"
}

variable "scaleway_zone" {
  type    = string
  default = "fr-par-1"
}

variable "cloudflare_zone_id" {
  type = string
}

variable "base_domain" {
  type = string
}

variable "accept_scaleway_tem_terms" {
  description = "Explicit confirmation that the Scaleway TEM terms have been reviewed and accepted for production. Must be true before apply."
  type        = bool

  validation {
    condition     = var.accept_scaleway_tem_terms
    error_message = "accept_scaleway_tem_terms must be true after the Scaleway TEM terms have been reviewed."
  }
}

variable "email_webhook_endpoint" {
  description = "Public HTTPS endpoint of nvbes-email-worker used for Scaleway SNS confirmation and signed TEM events."
  type        = string

  validation {
    condition     = startswith(var.email_webhook_endpoint, "https://")
    error_message = "email_webhook_endpoint must use HTTPS."
  }
}

variable "grafana_url" {
  description = "Grafana Cloud stack URL used by the alerting provisioning API."
  type        = string
}

variable "grafana_service_account_token" {
  description = "Grafana service account token limited to alerting provisioning."
  type        = string
  sensitive   = true
}

variable "grafana_loki_datasource_uid" {
  description = "UID of the production Loki datasource in Grafana Cloud."
  type        = string
}

variable "grafana_security_contact_point" {
  description = "Existing Grafana Alerting contact point for the security on-call rotation."
  type        = string
}

variable "postgres_password" {
  description = "Production PostgreSQL bootstrap password supplied through TF_VAR_postgres_password."
  type        = string
  sensitive   = true
}

variable "audit_anchor_kms_key_id" {
  description = "Protected asymmetric Key Manager key configured for SHA-256 signing."
  type        = string
}

variable "audit_archive_bucket_name" {
  description = "Globally unique bucket name owned by the security project."
  type        = string
}

variable "audit_archive_writer_access_key" {
  description = "Non-secret access key of the write-only identity created by the separate Security stack."
  type        = string
  sensitive   = true
}

variable "audit_anchor_kms_auth_token_secret_id" {
  description = "Production Secret Manager ID populated with the KMS-only signer API secret."
  type        = string
}

variable "audit_archive_writer_secret_id" {
  description = "Production Secret Manager ID populated with the external archive write-only API secret."
  type        = string
}

variable "account_database_url_secret_id" {
  description = "Protected Secret Manager ID containing the private Account PostgreSQL URL."
  type        = string
}

variable "files_bucket_name" {
  description = "Globally unique private customer-files bucket name."
  type        = string
}
