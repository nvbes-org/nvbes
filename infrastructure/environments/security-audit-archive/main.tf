locals {
  tags = [
    "nvbes",
    "environment:production",
    "account:security",
    "owner:security",
    "managed-by:terraform",
  ]
}

module "audit_archive" {
  source = "../../modules/scaleway-audit-archive"

  project_id      = var.security_project_id
  region          = var.scaleway_region
  name_prefix     = "nvbes-prod-security"
  bucket_name     = var.audit_archive_bucket_name
  retention_years = 7
  tags            = local.tags
}
