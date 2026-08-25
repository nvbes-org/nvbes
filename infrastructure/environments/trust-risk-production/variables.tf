variable "scaleway_project_id" {
  description = "Scaleway Project containing the production Trust/Risk resources."
  type        = string
}

variable "scaleway_organization_id" {
  description = "Scaleway Organization owning the production Trust/Risk IAM applications."
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
  description = "Optional production Private Network attached to the Trust/Risk container."
  type        = string
  default     = null
  nullable    = true

  validation {
    condition     = var.private_network_id == null || var.private_network_id == "" || can(regex("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$", var.private_network_id))
    error_message = "private_network_id must be a valid UUID or null/empty."
  }
}

variable "trust_risk_image" {
  description = "Signed immutable Trust/Risk image mirrored into the private Scaleway Registry."
  type        = string

  validation {
    condition     = can(regex("^rg\\.[a-z]{2}-[a-z-]+\\.scw\\.cloud/[a-z0-9-]+/nvbes-trust-risk-service@sha256:[0-9a-f]{64}$", var.trust_risk_image))
    error_message = "trust_risk_image must be an immutable private Scaleway Registry digest."
  }
}

variable "trust_risk_producer_policies" {
  description = "JSON producer policies used by authenticated signal, assessment and label APIs."
  type        = string
  sensitive   = true

  validation {
    condition     = can(jsondecode(var.trust_risk_producer_policies)) && length(jsondecode(var.trust_risk_producer_policies)) > 0
    error_message = "trust_risk_producer_policies must be a non-empty JSON array."
  }
}

variable "trust_risk_operator_tokens" {
  description = "JSON operator policies used by the protected operations API."
  type        = string
  sensitive   = true

  validation {
    condition     = can(jsondecode(var.trust_risk_operator_tokens)) && length(jsondecode(var.trust_risk_operator_tokens)) > 0
    error_message = "trust_risk_operator_tokens must be a non-empty JSON array."
  }
}

variable "trust_risk_metrics_token" {
  description = "Bearer token protecting the Trust/Risk metrics endpoint."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.trust_risk_metrics_token) >= 32 && !strcontains(lower(var.trust_risk_metrics_token), "change-me")
    error_message = "trust_risk_metrics_token must contain at least 32 non-placeholder characters."
  }
}

variable "trust_risk_sentry_dsn" {
  description = "Production Trust/Risk Sentry DSN supplied only by the protected deployment environment."
  type        = string
  sensitive   = true

  validation {
    condition = (
      var.trust_risk_sentry_dsn == trimspace(var.trust_risk_sentry_dsn) &&
      startswith(var.trust_risk_sentry_dsn, "https://") &&
      strcontains(var.trust_risk_sentry_dsn, "@")
    )
    error_message = "trust_risk_sentry_dsn must be a non-empty HTTPS Sentry DSN."
  }
}

variable "trust_risk_sentry_traces_sample_rate" {
  description = "Sentry transaction sampling rate for the production Trust/Risk runtime."
  type        = number
  default     = 0.1

  validation {
    condition     = var.trust_risk_sentry_traces_sample_rate >= 0 && var.trust_risk_sentry_traces_sample_rate <= 1
    error_message = "trust_risk_sentry_traces_sample_rate must be between 0 and 1."
  }
}

variable "grafana_url" {
  description = "Grafana Cloud stack URL used for Trust/Risk dashboard and alert provisioning."
  type        = string

  validation {
    condition     = startswith(var.grafana_url, "https://")
    error_message = "grafana_url must be HTTPS."
  }
}

variable "grafana_service_account_token" {
  description = "Grafana service account token limited to dashboard and alert provisioning."
  type        = string
  sensitive   = true
  ephemeral   = true
}

variable "grafana_prometheus_datasource_uid" {
  description = "UID of the Grafana Cloud Prometheus datasource containing Tempo span metrics."
  type        = string
}

variable "grafana_tempo_datasource_uid" {
  description = "UID of the Grafana Cloud Tempo datasource containing Trust/Risk traces."
  type        = string
}

variable "grafana_trust_risk_contact_point" {
  description = "Existing Grafana Alerting contact point for Trust/Risk incidents."
  type        = string
}

variable "grafana_otlp_endpoint" {
  description = "Grafana Cloud OTLP/gRPC endpoint used directly by the scale-to-zero runtime."
  type        = string

  validation {
    condition     = startswith(var.grafana_otlp_endpoint, "https://")
    error_message = "grafana_otlp_endpoint must be HTTPS."
  }
}

variable "grafana_otlp_authorization_header" {
  description = "Precomputed Basic authorization header for the Grafana Cloud OTLP gateway."
  type        = string
  sensitive   = true

  validation {
    condition = (
      startswith(var.grafana_otlp_authorization_header, "Basic ") &&
      !strcontains(var.grafana_otlp_authorization_header, "\n") &&
      !strcontains(var.grafana_otlp_authorization_header, "\r")
    )
    error_message = "grafana_otlp_authorization_header must be a single-line Basic authorization header."
  }
}
