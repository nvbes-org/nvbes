variable "scaleway_organization_id" {
  type        = string
  description = "Scaleway organization identifier."
}

variable "scaleway_project_id" {
  type        = string
  description = "Scaleway project identifier."
}

variable "scaleway_region" {
  type        = string
  description = "Scaleway deployment region."
  default     = "fr-par"
}

variable "scaleway_zone" {
  type        = string
  description = "Scaleway deployment zone."
  default     = "fr-par-1"
}

variable "platform_operations_image" {
  type        = string
  description = "Immutable digest-pinned Platform Operations container image."
}

variable "platform_operations_database_url" {
  type        = string
  sensitive   = true
  description = "Dedicated Operations database and restricted runtime role on the existing PostgreSQL instance."
}

variable "platform_operations_public_key_pem" {
  type        = string
  description = "RSA public key of the operator token issuer."
}

variable "platform_operations_issuer" {
  type        = string
  description = "Exact trusted issuer for MFA operator JWTs; audience is platform-operations."
}

variable "platform_operations_services" {
  type        = string
  default     = "[]"
  description = "JSON array of service and HTTPS base_url origins for readiness context."
}

variable "private_network_id" {
  type        = string
  description = "Optional Scaleway private network ID."
  default     = ""
}
