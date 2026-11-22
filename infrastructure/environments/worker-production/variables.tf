variable "scaleway_organization_id" {
  type        = string
  description = "Scaleway organization identifier."
}

variable "scaleway_project_id" {
  type        = string
  description = "Scaleway project identifier for production resources."
}

variable "scaleway_region" {
  type        = string
  description = "Scaleway region for regional resources."
  default     = "fr-par"
}

variable "scaleway_zone" {
  type        = string
  description = "Scaleway availability zone."
  default     = "fr-par-1"
}

variable "private_network_id" {
  type        = string
  description = "Optional Scaleway private network identifier."
  default     = ""
}

variable "worker_image" {
  type        = string
  description = "Container image reference for worker (with sha256 digest)."
}

variable "worker_database_url" {
  type        = string
  description = "Database URL for worker runtime."
  sensitive   = true
}

variable "worker_sentry_dsn" {
  type        = string
  description = "Sentry DSN for worker error reporting."
  sensitive   = true
  default     = null
}

variable "worker_sentry_traces_sample_rate" {
  type        = number
  description = "Sentry APM tracing sample rate."
  default     = 0.1
}

variable "grafana_otlp_endpoint" {
  type        = string
  description = "Grafana Cloud OTLP HTTP endpoint."
  default     = null
}

variable "grafana_otlp_authorization_header" {
  type        = string
  description = "Authorization header for Grafana Cloud OTLP."
  sensitive   = true
  default     = null
}
