locals {
  state_stacks = ["billing", "bootstrap", "email", "identity", "platform"]
  tags = [
    "nvbes",
    "environment:production",
    "managed-by:terraform",
    "purpose:terraform-state",
  ]
}

provider "scaleway" {
  profile    = var.scaleway_bootstrap_profile
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
  zone       = var.scaleway_zone
}

provider "github" {
  owner = var.github_owner
}

module "terraform_state" {
  source = "../../modules/terraform-state-backend"

  project_id   = var.scaleway_project_id
  region       = var.scaleway_region
  environment  = "production"
  bucket_name  = var.terraform_state_bucket
  state_stacks = local.state_stacks
  external_state_application_ids = {
    trust-risk = "4baaab67-b29e-4ba7-aed8-f8efce064a02"
  }
  tags = local.tags
}

module "ci_cache" {
  source = "../../modules/scaleway-ci-cache"

  organization_id              = var.scaleway_organization_id
  region                       = var.scaleway_region
  bucket_name                  = var.ci_cache_bucket_name
  credential_rotation_epoch    = var.ci_cache_credential_rotation_epoch
  credential_rotation_days     = var.ci_cache_credential_rotation_days
  credential_expiry_grace_days = var.ci_cache_credential_expiry_grace_days
  github_repository            = var.github_repository
  tags = [
    "nvbes",
    "environment:production",
    "managed-by:terraform",
    "purpose:ci-cache",
  ]
}
