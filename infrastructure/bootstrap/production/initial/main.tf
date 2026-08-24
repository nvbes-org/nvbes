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

module "terraform_state" {
  source = "../../../modules/terraform-state-backend"

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
