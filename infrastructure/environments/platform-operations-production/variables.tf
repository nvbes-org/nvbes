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

variable "platform_operations_token" {
  type        = string
  sensitive   = true
  description = "Secret operator token for Platform Operations cockpit."
}

variable "private_network_id" {
  type        = string
  description = "Optional Scaleway private network ID."
  default     = ""
}
