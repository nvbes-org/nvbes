variable "organization_id" {
  description = "Scaleway Organization owning the isolated CI cache project."
  type        = string
}

variable "region" {
  description = "Scaleway region hosting cache storage."
  type        = string
}

variable "project_name" {
  description = "Name of the dedicated Scaleway project."
  type        = string
  default     = "nvbes-ci-cache"
}

variable "registry_name" {
  description = "Name of the private BuildKit cache registry namespace."
  type        = string
  default     = "nvbes-prod-ci-cache"
}

variable "bucket_name" {
  description = "Globally unique Object Storage bucket for compiler and package caches."
  type        = string

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9.-]{1,61}[a-z0-9]$", var.bucket_name))
    error_message = "bucket_name must be a valid lowercase S3 bucket name."
  }
}

variable "object_retention_days" {
  description = "Maximum lifetime of reproducible cache objects."
  type        = number
  default     = 30

  validation {
    condition     = var.object_retention_days >= 7 && var.object_retention_days <= 90
    error_message = "object_retention_days must remain between 7 and 90 days."
  }
}

variable "credential_rotation_epoch" {
  description = "Stable RFC3339 epoch anchoring the staggered credential rotation."
  type        = string
  default     = "2026-09-01T00:00:00Z"

  validation {
    condition     = can(formatdate("YYYY-MM-DD'T'hh:mm:ssZ", var.credential_rotation_epoch))
    error_message = "credential_rotation_epoch must be an RFC3339 timestamp."
  }
}

variable "credential_rotation_days" {
  description = "Lifetime between rotations of each of the two credential slots."
  type        = number
  default     = 60

  validation {
    condition     = var.credential_rotation_days >= 30 && var.credential_rotation_days <= 90
    error_message = "credential_rotation_days must remain between 30 and 90 days."
  }
}

variable "credential_expiry_grace_days" {
  description = "Expiry grace after a slot rotation boundary."
  type        = number
  default     = 7

  validation {
    condition     = var.credential_expiry_grace_days >= 1 && var.credential_expiry_grace_days <= 14
    error_message = "credential_expiry_grace_days must remain between 1 and 14 days."
  }
}

variable "github_repository" {
  description = "GitHub repository receiving the protected CI cache environment."
  type        = string
}

variable "github_environment" {
  description = "GitHub environment exposing cache credentials to trusted build jobs."
  type        = string
  default     = "production-ci-cache"
}

variable "github_branch_environment" {
  description = "GitHub environment exposing prefix-restricted credentials to branch push jobs."
  type        = string
  default     = "branch-ci-cache"
}

variable "tags" {
  description = "Tags applied to supported Scaleway cache resources."
  type        = list(string)
  default     = []
}
