variable "scaleway_project_id" {
  description = "Scaleway project ID for development."
  type        = string
}

variable "scaleway_region" {
  description = "Scaleway region for development."
  type        = string
  default     = "fr-par"
}

variable "scaleway_zone" {
  description = "Scaleway zone for development."
  type        = string
  default     = "fr-par-1"
}

variable "cloudflare_zone_id" {
  description = "Cloudflare zone ID hosting the development records."
  type        = string
}

variable "base_domain" {
  description = "Base DNS zone, for example nvbes.fr."
  type        = string
}

variable "ssh_allowed_ips" {
  description = "CIDRs allowed to SSH to development compute."
  type        = list(string)
  default     = []
}

variable "postgres_password" {
  description = "Development PostgreSQL password. Supply via TF_VAR_postgres_password."
  type        = string
  sensitive   = true
}
