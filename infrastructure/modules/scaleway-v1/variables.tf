variable "project_id" {
  description = "Scaleway project ID for the environment."
  type        = string
}

variable "region" {
  description = "Scaleway region."
  type        = string
  default     = "fr-par"
}

variable "zone" {
  description = "Scaleway availability zone."
  type        = string
  default     = "fr-par-1"
}

variable "environment" {
  description = "Environment name."
  type        = string
}

variable "name_prefix" {
  description = "Prefix used for resource names."
  type        = string
}

variable "private_subnet" {
  description = "Private network IPv4 CIDR."
  type        = string
}

variable "ssh_allowed_ips" {
  description = "CIDRs allowed to SSH to compute instances."
  type        = list(string)
  default     = []
}

variable "api_instance_type" {
  description = "Scaleway Instance type for the API node."
  type        = string
}

variable "worker_instance_type" {
  description = "Scaleway Instance type for the worker node."
  type        = string
}

variable "instance_image" {
  description = "Scaleway Instance image slug."
  type        = string
  default     = "ubuntu_jammy"
}

variable "rdb_node_type" {
  description = "Managed PostgreSQL node type."
  type        = string
}

variable "postgres_engine" {
  description = "Managed PostgreSQL engine."
  type        = string
  default     = "PostgreSQL-15"
}

variable "postgres_user" {
  description = "Initial PostgreSQL username."
  type        = string
  default     = "nvbes"
}

variable "postgres_password" {
  description = "Initial PostgreSQL password. Provide from a secure TF variable source."
  type        = string
  sensitive   = true
}

variable "postgres_backup_retention_days" {
  description = "Managed PostgreSQL backup retention in days."
  type        = number
}

variable "bucket_name" {
  description = "Globally unique Object Storage bucket name."
  type        = string
}

variable "bucket_cors_allowed_origins" {
  description = "Allowed origins for signed upload/download browser flows."
  type        = list(string)
}

variable "tags" {
  description = "Common tags."
  type        = list(string)
  default     = []
}

variable "cloudflare_zone_id" {
  description = "Cloudflare zone ID for TLS and DNS configuration."
  type        = string
  default     = null
}

variable "domain" {
  description = "Primary domain name for TLS certificate."
  type        = string
  default     = null
}

variable "enable_legacy_vm_worker" {
  description = "Set true only if legacy 24/7 VM worker instance is required. Serverless Jobs/Containers are preferred."
  type        = bool
  default     = false
}
