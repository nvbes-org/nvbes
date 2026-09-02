variable "scaleway_project_id" {
  description = "Dedicated Scaleway project owning production Terraform state."
  type        = string
}

variable "scaleway_organization_id" {
  description = "Scaleway Organization in which the isolated CI cache project is created."
  type        = string
}

variable "scaleway_bootstrap_profile" {
  description = "Optional local Scaleway profile used only by bootstrap administrators."
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

variable "terraform_state_bucket" {
  description = "Globally unique bucket shared by production Terraform stacks."
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]$", var.terraform_state_bucket))
    error_message = "terraform_state_bucket must be a valid lowercase S3 bucket name."
  }
}

variable "ci_cache_bucket_name" {
  description = "Globally unique Object Storage bucket for reproducible CI caches."
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]$", var.ci_cache_bucket_name))
    error_message = "ci_cache_bucket_name must be a valid lowercase S3 bucket name."
  }
}

variable "ci_cache_credential_rotation_epoch" {
  description = "Stable RFC3339 epoch anchoring staggered CI cache key rotation."
  type        = string
  default     = "2026-09-01T00:00:00Z"
}

variable "ci_cache_credential_rotation_days" {
  description = "Rotation interval for each CI cache credential slot."
  type        = number
  default     = 60
}

variable "ci_cache_credential_expiry_grace_days" {
  description = "Expiry grace after each credential rotation boundary."
  type        = number
  default     = 7
}

variable "github_owner" {
  description = "GitHub organization owning the nvbes repository."
  type        = string
  default     = "nvbes-org"
}

variable "github_repository" {
  description = "GitHub repository receiving the CI cache environment."
  type        = string
  default     = "nvbes"
}
