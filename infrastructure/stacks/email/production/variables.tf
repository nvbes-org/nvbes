variable "scaleway_project_id" {
  description = "Scaleway project ID owning the production email stack."
  type        = string
}

variable "scaleway_production_profile" {
  description = "Optional local Scaleway profile. CI authenticates through SCW_ACCESS_KEY and SCW_SECRET_KEY."
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
  description = "Cloudflare zone ID authoritative for the production email domain."
  type        = string
}

variable "base_domain" {
  description = "Apex domain; transactional delivery uses notify.<base_domain>."
  type        = string
}

variable "accept_scaleway_tem_terms" {
  description = "Explicit confirmation that Scaleway TEM terms were reviewed and accepted."
  type        = bool

  validation {
    condition     = var.accept_scaleway_tem_terms
    error_message = "accept_scaleway_tem_terms must be true after the Scaleway TEM terms have been reviewed."
  }
}

variable "email_worker_source_image" {
  description = "Signed GHCR email-worker image pinned by OCI digest and mirrored into the private Scaleway registry."
  type        = string

  validation {
    condition = can(regex(
      "^ghcr\\.io/nvbes-org/nvbes-email-worker@sha256:[0-9a-f]{64}$",
      var.email_worker_source_image
    ))
    error_message = "email_worker_source_image must be the nvbes email-worker GHCR image pinned by sha256 digest."
  }
}

variable "email_worker_database_url" {
  description = "Production PostgreSQL URL consumed by email-worker."
  type        = string
  sensitive   = true
  ephemeral   = true

  validation {
    condition     = can(regex("^postgres(ql)?://", var.email_worker_database_url))
    error_message = "email_worker_database_url must be a PostgreSQL URL."
  }
}

variable "email_worker_producer_tokens" {
  description = "Comma-separated producer=token authentication map consumed by email-worker."
  type        = string
  sensitive   = true
  ephemeral   = true

  validation {
    condition     = trimspace(var.email_worker_producer_tokens) != ""
    error_message = "email_worker_producer_tokens must not be empty."
  }
}

variable "email_worker_data_encryption_key" {
  description = "Base64-encoded 32-byte key encrypting retained email payloads."
  type        = string
  sensitive   = true
  ephemeral   = true

  validation {
    condition     = can(regex("^[A-Za-z0-9+/]{43}=$", var.email_worker_data_encryption_key))
    error_message = "email_worker_data_encryption_key must encode exactly 32 bytes in base64."
  }
}

variable "email_worker_recipient_hmac_key" {
  description = "Base64-encoded 32-byte key hashing email recipients."
  type        = string
  sensitive   = true
  ephemeral   = true

  validation {
    condition     = can(regex("^[A-Za-z0-9+/]{43}=$", var.email_worker_recipient_hmac_key))
    error_message = "email_worker_recipient_hmac_key must encode exactly 32 bytes in base64."
  }
}

variable "email_worker_scaleway_secret_key" {
  description = "Dedicated least-privilege Scaleway TEM credential consumed by email-worker."
  type        = string
  sensitive   = true
  ephemeral   = true

  validation {
    condition     = trimspace(var.email_worker_scaleway_secret_key) != ""
    error_message = "email_worker_scaleway_secret_key must not be empty."
  }
}

variable "email_worker_observability_internal_token" {
  description = "Bearer token protecting the email-worker metrics endpoint."
  type        = string
  sensitive   = true
  ephemeral   = true

  validation {
    condition = (
      length(var.email_worker_observability_internal_token) >= 32 &&
      !strcontains(lower(var.email_worker_observability_internal_token), "change-me")
    )
    error_message = "email_worker_observability_internal_token must contain at least 32 non-placeholder characters."
  }
}

variable "email_worker_runtime_secret_version" {
  description = "Monotonic deployment trigger incremented whenever a non-Sentry runtime secret rotates."
  type        = number
  default     = 1

  validation {
    condition = (
      var.email_worker_runtime_secret_version >= 1 &&
      floor(var.email_worker_runtime_secret_version) == var.email_worker_runtime_secret_version
    )
    error_message = "email_worker_runtime_secret_version must be a positive integer."
  }
}

variable "grafana_url" {
  description = "Grafana Cloud stack URL used for email observability provisioning."
  type        = string
}

variable "grafana_service_account_token" {
  description = "Grafana service account token limited to dashboard and alerting provisioning."
  type        = string
  sensitive   = true
  ephemeral   = true
}

variable "grafana_prometheus_datasource_uid" {
  description = "UID of the production Prometheus datasource in Grafana Cloud."
  type        = string
}

variable "grafana_email_contact_point" {
  description = "Existing Grafana Alerting contact point for the email platform on-call rotation."
  type        = string
}

variable "email_sentry_dsn" {
  description = "Production email-worker Sentry DSN; supplied only during plan/apply."
  type        = string
  sensitive   = true
  ephemeral   = true

  validation {
    condition = (
      var.email_sentry_dsn == trimspace(var.email_sentry_dsn) &&
      startswith(var.email_sentry_dsn, "https://") &&
      strcontains(var.email_sentry_dsn, "@")
    )
    error_message = "email_sentry_dsn must be a non-empty HTTPS Sentry DSN."
  }
}

variable "email_sentry_dsn_rotation" {
  description = "Blue/green Secret Manager slots used to retain N and N-1 during an email-worker Sentry DSN rollout."
  type = object({
    active_slot     = string
    blue_version    = number
    green_version   = number
    retain_previous = bool
  })
  default = {
    active_slot     = "blue"
    blue_version    = 1
    green_version   = 0
    retain_previous = true
  }

  validation {
    condition = (
      contains(["blue", "green"], var.email_sentry_dsn_rotation.active_slot) &&
      var.email_sentry_dsn_rotation.blue_version >= 0 &&
      floor(var.email_sentry_dsn_rotation.blue_version) == var.email_sentry_dsn_rotation.blue_version &&
      var.email_sentry_dsn_rotation.green_version >= 0 &&
      floor(var.email_sentry_dsn_rotation.green_version) == var.email_sentry_dsn_rotation.green_version
    )
    error_message = "Sentry rotation must use blue or green with non-negative integer versions."
  }

  validation {
    condition = var.email_sentry_dsn_rotation.active_slot == "blue" ? (
      var.email_sentry_dsn_rotation.blue_version >= 1 &&
      var.email_sentry_dsn_rotation.blue_version > var.email_sentry_dsn_rotation.green_version
      ) : (
      var.email_sentry_dsn_rotation.green_version >= 1 &&
      var.email_sentry_dsn_rotation.green_version > var.email_sentry_dsn_rotation.blue_version
    )
    error_message = "The active Sentry slot version must be positive and newer than the inactive slot."
  }
}

variable "email_sentry_traces_sample_rate" {
  description = "Sentry transaction sampling rate injected into email-worker."
  type        = number
  default     = 0

  validation {
    condition = (
      var.email_sentry_traces_sample_rate >= 0 &&
      var.email_sentry_traces_sample_rate <= 1
    )
    error_message = "email_sentry_traces_sample_rate must be between 0 and 1."
  }
}
