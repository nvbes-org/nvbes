variable "scaleway_project_id" {
  type        = string
  description = "Scaleway project identifier hosting production billing resources."
}

variable "scaleway_region" {
  type        = string
  default     = "fr-par"
  description = "Scaleway region targeted for production billing."
}

variable "scaleway_zone" {
  type        = string
  default     = "fr-par-1"
  description = "Scaleway availability zone."
}

variable "environment" {
  type        = string
  default     = "production"
  description = "Deployment environment name."
}

variable "billing_image" {
  type        = string
  description = "Pinned container image reference including registry and digest."
}

variable "private_network_id" {
  type        = string
  default     = null
  description = "Optional Scaleway private network ID."
}

variable "stripe_secret_key" {
  type        = string
  sensitive   = true
  description = "Stripe secret key in test mode (must start with sk_test_ or rk_test_)."
}

variable "stripe_webhook_secret" {
  type        = string
  sensitive   = true
  description = "Stripe webhook signing secret in test mode (must start with whsec_)."
}

variable "billing_metrics_token" {
  type        = string
  sensitive   = true
  description = "Bearer token for scraping Prometheus metrics."
}

variable "billing_operator_token" {
  type        = string
  sensitive   = true
  description = "Bearer token for operator API access."
}

variable "grafana_otlp_endpoint" {
  type        = string
  default     = ""
  description = "OTLP telemetry collector endpoint."
}

variable "grafana_otlp_authorization_header" {
  type        = string
  default     = ""
  sensitive   = true
  description = "Basic or Bearer authorization header for OTLP ingestion."
}

variable "billing_sentry_dsn" {
  type        = string
  default     = ""
  sensitive   = true
  description = "Sentry DSN for error tracking."
}

variable "billing_sentry_traces_sample_rate" {
  type        = number
  default     = 0.1
  description = "Sentry transaction sample rate."
}
