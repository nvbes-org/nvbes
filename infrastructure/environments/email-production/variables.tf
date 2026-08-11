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
  description = "Existing production Private Network attached to email containers for egress."
  type        = string
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
