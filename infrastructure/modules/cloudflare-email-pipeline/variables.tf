variable "cloudflare_account_id" {
  type        = string
  description = "Account ID Cloudflare"
}

variable "cloudflare_zone_id" {
  type        = string
  description = "Zone ID DNS Cloudflare pour le domaine principal"
}

variable "domain_name" {
  type        = string
  description = "Domaine d'envoi des emails (ex: nvbes.fr)"
  default     = "nvbes.fr"
}

variable "scaleway_project_id" {
  type        = string
  description = "Project ID Scaleway TEM"
  sensitive   = true
}

variable "scaleway_secret_key" {
  type        = string
  description = "API Secret Key Scaleway TEM"
  sensitive   = true
}

variable "environment" {
  type        = string
  description = "Environnement cible (development, staging, production)"
  default     = "production"
}
