variable "scaleway_project_id" {
  description = "Scaleway Project containing production Identity resources."
  type        = string
}

variable "scaleway_organization_id" {
  description = "Scaleway Organization owning production Identity IAM applications."
  type        = string

  validation {
    condition     = can(regex("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", var.scaleway_organization_id))
    error_message = "scaleway_organization_id must be a valid UUID."
  }
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
  description = "Optional production Private Network attached to Identity."
  type        = string
  default     = null
  nullable    = true
}

variable "identity_image" {
  description = "Signed immutable Identity image in the private Scaleway Registry."
  type        = string

  validation {
    condition     = can(regex("^rg\\.[a-z]{2}-[a-z-]+\\.scw\\.cloud/[a-z0-9-]+/nvbes-identity-service@sha256:[0-9a-f]{64}$", var.identity_image))
    error_message = "identity_image must be an immutable private Scaleway Registry digest."
  }
}

variable "identity_mfa_encryption_key" {
  description = "Base64-encoded 32-byte active MFA encryption key."
  type        = string
  sensitive   = true

  validation {
    condition     = can(base64decode(var.identity_mfa_encryption_key)) && length(base64decode(var.identity_mfa_encryption_key)) == 32
    error_message = "identity_mfa_encryption_key must encode exactly 32 bytes."
  }
}

variable "identity_mfa_key_version" {
  description = "Positive version of the active MFA encryption key."
  type        = number
  default     = 1

  validation {
    condition     = var.identity_mfa_key_version >= 1 && floor(var.identity_mfa_key_version) == var.identity_mfa_key_version
    error_message = "identity_mfa_key_version must be a positive integer."
  }
}

variable "identity_metrics_token" {
  description = "Bearer token protecting Identity Prometheus metrics."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.identity_metrics_token) >= 32 && !strcontains(lower(var.identity_metrics_token), "change-me")
    error_message = "identity_metrics_token must contain at least 32 non-placeholder characters."
  }
}

variable "identity_sentry_dsn" {
  description = "Production Identity Sentry DSN."
  type        = string
  sensitive   = true

  validation {
    condition     = startswith(var.identity_sentry_dsn, "https://") && strcontains(var.identity_sentry_dsn, "@")
    error_message = "identity_sentry_dsn must be a non-empty HTTPS Sentry DSN."
  }
}

variable "identity_sentry_traces_sample_rate" {
  type    = number
  default = 0.1

  validation {
    condition     = var.identity_sentry_traces_sample_rate >= 0 && var.identity_sentry_traces_sample_rate <= 1
    error_message = "identity_sentry_traces_sample_rate must be between 0 and 1."
  }
}

variable "grafana_otlp_endpoint" {
  description = "Grafana Cloud OTLP endpoint used by Identity."
  type        = string

  validation {
    condition     = startswith(var.grafana_otlp_endpoint, "https://")
    error_message = "grafana_otlp_endpoint must be HTTPS."
  }
}

variable "grafana_otlp_authorization_header" {
  description = "Precomputed Basic authorization header for Grafana Cloud OTLP."
  type        = string
  sensitive   = true

  validation {
    condition     = startswith(var.grafana_otlp_authorization_header, "Basic ") && !strcontains(var.grafana_otlp_authorization_header, "\n") && !strcontains(var.grafana_otlp_authorization_header, "\r")
    error_message = "grafana_otlp_authorization_header must be a single-line Basic header."
  }
}
