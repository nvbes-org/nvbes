locals {
  environment = "production"
  name_prefix = "nvbes-prod"
  tags = [
    "nvbes",
    "environment:production",
    "product:email",
    "managed-by:terraform",
  ]
}

provider "scaleway" {
  profile    = var.scaleway_production_profile
  project_id = var.scaleway_project_id
  region     = var.scaleway_region
  zone       = var.scaleway_zone
}

provider "cloudflare" {}

provider "grafana" {
  url  = var.grafana_url
  auth = var.grafana_service_account_token
}
