variable "scaleway_project_id" {
  description = "Dedicated Scaleway project owning production Terraform state."
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
