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
  description = "Short-lived CIDRs allowed to SSH during an approved JIT window. Keep empty by default."
  type        = list(string)
  default     = []
}

variable "enable_jit_ssh" {
  description = "Temporarily enables SSH rules for approved JIT CIDRs."
  type        = bool
  default     = false
}

variable "edge_allowed_ipv4_cidrs" {
  description = "Trusted reverse-proxy CIDRs allowed to reach the API origin."
  type        = list(string)
  default     = []
}

variable "egress_https_allowed_cidrs" {
  description = "CIDRs reachable over HTTPS by API and worker workloads."
  type        = list(string)
  default     = ["0.0.0.0/0"]
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

variable "enable_external_audit_archive" {
  description = "Provision the external WORM audit archive and dedicated writer identity."
  type        = bool
  default     = false
}

variable "audit_archive_bucket_name" {
  description = "Globally unique bucket name for signed audit anchors."
  type        = string
  default     = null
}

variable "audit_archive_writer_access_key" {
  description = "Access key for the write-only identity owned by the external Security account."
  type        = string
  default     = null
  sensitive   = true
}

variable "audit_anchor_kms_key_id" {
  description = "Protected asymmetric Scaleway KMS key used only to sign audit anchor digests."
  type        = string
  default     = null
}

variable "audit_anchor_kms_auth_token_secret_id" {
  description = "Secret Manager ID containing the production KMS signer API secret."
  type        = string
  default     = null
}

variable "audit_archive_writer_secret_id" {
  description = "Secret Manager ID containing the external Security-account Object Storage writer secret."
  type        = string
  default     = null
}

variable "account_database_url_secret_id" {
  description = "Secret Manager ID containing the private Account PostgreSQL URL for Serverless Jobs."
  type        = string
  default     = null
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
