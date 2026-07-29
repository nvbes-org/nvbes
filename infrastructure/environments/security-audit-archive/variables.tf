variable "security_project_id" {
  description = "Project ID in the Security-owned Scaleway account."
  type        = string
}

variable "security_scaleway_profile" {
  description = "Scaleway profile authorized only in the Security archive account."
  type        = string
}

variable "scaleway_region" {
  type    = string
  default = "fr-par"
}

variable "audit_archive_bucket_name" {
  description = "Globally unique immutable audit evidence bucket name."
  type        = string
}
