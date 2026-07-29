terraform {
  required_version = ">= 1.11.0"

  required_providers {
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5.19"
    }
    grafana = {
      source  = "grafana/grafana"
      version = "~> 4.37"
    }
    scaleway = {
      source  = "scaleway/scaleway"
      version = "~> 2.79"
    }
  }
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
