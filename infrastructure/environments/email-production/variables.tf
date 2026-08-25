variable "scaleway_project_id" {
  description = "Scaleway Project containing the production email resources."
  type        = string
}

variable "scaleway_region" {
  type    = string
  default = "fr-par"
}

variable "scaleway_zone" {
  type    = string
  default = "fr-par-1"
}

variable "private_network_id" {
  description = "Optional production Private Network attached to email containers for private egress."
  type        = string
  default     = null
  nullable    = true

  validation {
    condition     = var.private_network_id == null || var.private_network_id == "" || can(regex("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", var.private_network_id))
    error_message = "private_network_id must be a valid UUID or null/empty."
  }
}

variable "cloudflare_zone_id" {
  description = "Cloudflare zone receiving the transactional email DNS records."
  type        = string
}

variable "base_domain" {
  description = "Apex domain; transactional email uses notify.<base_domain>."
  type        = string
}

variable "accept_scaleway_tem_terms" {
  description = "Explicit confirmation that the Scaleway TEM terms were reviewed and accepted."
  type        = bool

  validation {
    condition     = var.accept_scaleway_tem_terms
    error_message = "accept_scaleway_tem_terms must be true after review of the Scaleway TEM terms."
  }
}

variable "email_webhook_endpoint" {
  description = "Optional HTTPS override for TEM events; defaults to the ingress container endpoint."
  type        = string
  default     = null
  nullable    = true

  validation {
    condition     = var.email_webhook_endpoint == null || startswith(var.email_webhook_endpoint, "https://")
    error_message = "email_webhook_endpoint must use HTTPS."
  }
}

variable "email_worker_image" {
  description = "Signed immutable email-worker image mirrored into the private Scaleway registry."
  type        = string

  validation {
    condition     = can(regex("^rg\\.[a-z]{2}-[a-z-]+\\.scw\\.cloud/[a-z0-9-]+/nvbes-email-worker@sha256:[0-9a-f]{64}$", var.email_worker_image))
    error_message = "email_worker_image must be an immutable private Scaleway Registry nvbes-email-worker@sha256:<digest> reference."
  }
}

variable "email_producer_tokens" {
  description = "Comma-separated producer-to-token bindings accepted by the email gRPC API."
  type        = string
  sensitive   = true
}

variable "email_data_encryption_key" {
  description = "Base64-encoded 32-byte key encrypting email payloads at rest."
  type        = string
  sensitive   = true
}

variable "email_recipient_hmac_key" {
  description = "Base64-encoded 32-byte key hashing recipients for suppression matching."
  type        = string
  sensitive   = true
}

variable "email_observability_internal_token" {
  description = "Bearer token protecting the email-worker metrics endpoint."
  type        = string
  sensitive   = true

  validation {
    condition = (
      length(var.email_observability_internal_token) >= 32 &&
      !strcontains(lower(var.email_observability_internal_token), "change-me")
    )
    error_message = "email_observability_internal_token must contain at least 32 non-placeholder characters."
  }
}

variable "email_sentry_dsn" {
  description = "HTTPS DSN for the dedicated production email-worker Sentry project."
  type        = string
  sensitive   = true

  validation {
    condition = (
      var.email_sentry_dsn == trimspace(var.email_sentry_dsn) &&
      startswith(var.email_sentry_dsn, "https://") &&
      strcontains(var.email_sentry_dsn, "@")
    )
    error_message = "email_sentry_dsn must be a non-empty HTTPS Sentry DSN."
  }
}

variable "email_sentry_traces_sample_rate" {
  description = "Sentry transaction sampling rate for production email-worker runtimes."
  type        = number
  default     = 0.1

  validation {
    condition = (
      var.email_sentry_traces_sample_rate >= 0 &&
      var.email_sentry_traces_sample_rate <= 1
    )
    error_message = "email_sentry_traces_sample_rate must be between 0 and 1."
  }
}

variable "grafana_url" {
  description = "Grafana Cloud stack URL used to provision Email dashboards and alert rules."
  type        = string

  validation {
    condition     = startswith(var.grafana_url, "https://")
    error_message = "grafana_url must use HTTPS."
  }
}

variable "grafana_service_account_token" {
  description = "Grafana service account token limited to dashboard and alert provisioning."
  type        = string
  sensitive   = true
  ephemeral   = true
  default     = null
  nullable    = true
}

variable "grafana_prometheus_datasource_uid" {
  description = "UID of the production Prometheus datasource in Grafana Cloud."
  type        = string
}

variable "grafana_email_contact_point" {
  description = "Existing Grafana Alerting contact point for Email incidents."
  type        = string
}

variable "scaleway_email_secret_key" {
  description = "Scaleway TEM secret key limited to transactional email sending."
  type        = string
  sensitive   = true
}

variable "email_sns_ca_bundle_pem" {
  description = "PEM CA bundle used to validate Scaleway SNS signing certificates."
  type        = string
  sensitive   = true
}
