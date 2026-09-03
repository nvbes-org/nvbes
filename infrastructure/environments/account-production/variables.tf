variable "scaleway_project_id" {
  description = "Scaleway Project containing production Account resources."
  type        = string
}

variable "scaleway_organization_id" {
  description = "Scaleway Organization owning production Account IAM applications."
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
  description = "Optional production Private Network attached to Account."
  type        = string
  default     = null
  nullable    = true
}

variable "account_image" {
  description = "Signed immutable Account image in the private Scaleway Registry."
  type        = string

  validation {
    condition     = can(regex("^rg\\.[a-z]{2}-[a-z-]+\\.scw\\.cloud/[a-z0-9-]+/nvbes-account-service@sha256:[0-9a-f]{64}$", var.account_image))
    error_message = "account_image must be an immutable private Scaleway Registry digest."
  }
}

variable "account_metrics_token" {
  description = "Bearer token protecting Account Prometheus metrics."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.account_metrics_token) >= 32 && !strcontains(lower(var.account_metrics_token), "change-me")
    error_message = "account_metrics_token must contain at least 32 non-placeholder characters."
  }
}

variable "identity_token_issuer" {
  description = "Expected Identity JWT issuer."
  type        = string
  default     = "https://identity.nvbes.com"

  validation {
    condition     = startswith(var.identity_token_issuer, "https://")
    error_message = "identity_token_issuer must be HTTPS."
  }
}

variable "identity_token_audience" {
  description = "Expected Identity JWT audience."
  type        = string
  default     = "nvbes-account"
}

variable "identity_token_key_id" {
  description = "Key ID for active Identity RS256 token verification."
  type        = string
}

variable "identity_token_public_key_pem" {
  description = "PEM-encoded RSA public key from Identity."
  type        = string
  sensitive   = true

  validation {
    condition     = strcontains(var.identity_token_public_key_pem, "BEGIN PUBLIC KEY")
    error_message = "identity_token_public_key_pem must contain BEGIN PUBLIC KEY."
  }
}

variable "account_sentry_dsn" {
  description = "Production Account Sentry DSN."
  type        = string
  sensitive   = true

  validation {
    condition     = startswith(var.account_sentry_dsn, "https://") && strcontains(var.account_sentry_dsn, "@")
    error_message = "account_sentry_dsn must be a non-empty HTTPS Sentry DSN."
  }
}

variable "account_sentry_traces_sample_rate" {
  type    = number
  default = 0.1

  validation {
    condition     = var.account_sentry_traces_sample_rate >= 0 && var.account_sentry_traces_sample_rate <= 1
    error_message = "account_sentry_traces_sample_rate must be between 0 and 1."
  }
}

variable "grafana_otlp_endpoint" {
  description = "Grafana Cloud OTLP endpoint used by Account."
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
